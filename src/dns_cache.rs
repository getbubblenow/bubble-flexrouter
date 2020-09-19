#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::future::Future;
use std::net::{SocketAddr, IpAddr};
use std::sync::Arc;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::task::{self, Poll};

use hyper::client::connect::dns::Name;

use log::{trace, debug, error};

use lru::LruCache;

use tower::Service;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};
use trust_dns_resolver::error::{ResolveError};

#[derive(Debug)]
pub enum DnsResolveError {
    ResolutionFailure (ResolveError),
    DnsNoRecordsFound,
    DnsUnknownError,
    InterruptedError (Error)
}

impl std::fmt::Display for DnsResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for DnsResolveError {}

pub async fn create_resolver(dns1_sock: SocketAddr, dns2_sock: SocketAddr) -> TokioAsyncResolver {
    let mut resolver_config: ResolverConfig = ResolverConfig::new();

    resolver_config.add_name_server(NameServerConfig {
        socket_addr: dns1_sock,
        protocol: Protocol::Udp,
        tls_dns_name: None
    });
    resolver_config.add_name_server(NameServerConfig {
        socket_addr: dns1_sock,
        protocol: Protocol::Tcp,
        tls_dns_name: None
    });
    resolver_config.add_name_server(NameServerConfig {
        socket_addr: dns2_sock,
        protocol: Protocol::Udp,
        tls_dns_name: None
    });
    resolver_config.add_name_server(NameServerConfig {
        socket_addr: dns2_sock,
        protocol: Protocol::Tcp,
        tls_dns_name: None
    });
    TokioAsyncResolver::tokio(resolver_config, ResolverOpts::default()).await.unwrap()
}

pub async fn resolve_with_cache(host: &str,
                                resolver: &TokioAsyncResolver,
                                resolver_cache: Arc<Mutex<LruCache<String, String>>>) -> Result<String, DnsResolveError> {
    let host_string = String::from(host);
    let mut guard = resolver_cache.lock().await;
    let found = (*guard).get(&host_string);

    if found.is_none() {
        trace!("resolve_with_cache: host={} not in cache, resolving...", String::from(host_string.as_str()));
        let lookup_result = resolver.lookup_ip(host).await;
        if lookup_result.is_err() {
            let err = lookup_result.err();
            if err.is_some() {
                Err(DnsResolveError::ResolutionFailure(err.unwrap()))
            } else {
                Err(DnsResolveError::DnsUnknownError)
            }
        } else {
            let ip_result = lookup_result.unwrap();
            let first_result = ip_result.iter().next();
            if first_result.is_none() {
                error!("resolve_with_cache: {} - no records found", String::from(host_string.as_str()));
                Err(DnsResolveError::DnsNoRecordsFound)
            } else {
                let resolved_ip = format!("{}", first_result.unwrap());
                (*guard).put(String::from(host_string.as_str()), resolved_ip.to_string());
                debug!("resolve_with_cache: resolved {} -> {}", String::from(host_string.as_str()), &resolved_ip);
                Ok(resolved_ip)
            }
        }
    } else {
        let found = found.unwrap();
        trace!("resolve_with_cache: host={} found in cache, returning: {}", host_string, found);
        Ok(String::from(found))
    }
}

#[derive(Clone)]
pub struct CacheResolver {
    _resolver: Arc<TokioAsyncResolver>,
    _cache: Arc<Mutex<LruCache<String, String>>>
}

impl CacheResolver {
    pub fn new(resolver: Arc<TokioAsyncResolver>, cache: Arc<Mutex<LruCache<String, String>>>) -> Self {
        CacheResolver { _resolver: resolver, _cache: cache }
    }
}

pub struct IpAddrs {
    iter: std::vec::IntoIter<SocketAddr>,
}

pub struct CacheAddrs {
    inner: IpAddrs,
}

pub struct CacheFuture {
    inner: JoinHandle<Result<IpAddrs, DnsResolveError>>
}

pub async fn resolve_to_result(host: String,
                               resolver: Arc<TokioAsyncResolver>,
                               cache: Arc<Mutex<LruCache<String, String>>>) -> Result<IpAddrs, DnsResolveError> {
    let resolve_result = resolve_with_cache(host.as_str(), &resolver, cache).await;
    if resolve_result.is_err() {
        Err(resolve_result.err().unwrap())
    } else {
        let ip = resolve_result.unwrap();
        let ip_addr: IpAddr = ip.parse().unwrap();
        let sock = SocketAddr::new(ip_addr, 0);
        Ok(IpAddrs { iter: vec![sock].into_iter() })
    }
}

impl Service<Name> for CacheResolver {
    type Response = CacheAddrs;
    type Error = DnsResolveError;
    type Future = CacheFuture;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), DnsResolveError>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, name: Name) -> CacheFuture {
        debug!("CacheResolver.call resolving host={:?}", name.as_str());
        let resolver: Arc<TokioAsyncResolver> = self._resolver.clone();
        let cache: Arc<Mutex<LruCache<String, String>>> = self._cache.clone();
        let addrs = tokio::task::spawn(
            resolve_to_result(String::from(name.as_str()), resolver, cache)
        );
        CacheFuture { inner: addrs }
    }
}

impl Future for CacheFuture {
    type Output = Result<CacheAddrs, DnsResolveError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map(|res| match res {
            Ok(Ok(addrs)) => Ok(CacheAddrs { inner: addrs }),
            Ok(Err(err)) => Err(err),
            Err(join_err) => {
                if join_err.is_cancelled() {
                    Err(DnsResolveError::InterruptedError(Error::new(ErrorKind::Interrupted, join_err)))
                } else {
                    panic!("gai background task failed: {:?}", join_err)
                }
            }
        })
    }
}

impl Iterator for IpAddrs {
    type Item = SocketAddr;
    #[inline]
    fn next(&mut self) -> Option<SocketAddr> {
        self.iter.next()
    }
}

impl Iterator for CacheAddrs {
    type Item = IpAddr;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|sa| sa.ip())
    }
}
