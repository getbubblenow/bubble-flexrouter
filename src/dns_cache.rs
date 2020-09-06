#![deny(warnings)]
/**
 * Copyright (c) 2020 Bubble, Inc.  All rights reserved.
 * For personal (non-commercial) use, see license: https://getbubblenow.com/bubble-license/
 */

use std::future::Future;
use std::net::{SocketAddr, IpAddr};
use std::sync::Arc;
use std::io::Error;
use std::pin::Pin;
use std::task::{self, Poll};

use hyper::client::connect::dns::Name;

use log::debug;

use lru::LruCache;

use tower::Service;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};

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
                                resolver_cache: Arc<Mutex<LruCache<String, String>>>) -> String {
    let host_string = String::from(host);
    let mut guard = resolver_cache.lock().await;
    let found = (*guard).get(&host_string);

    if found.is_none() {
        debug!("resolve_with_cache: host={} not in cache, resolving...", host_string);
        let resolved_ip = format!("{}", resolver.lookup_ip(host).await.unwrap().iter().next().unwrap());
        (*guard).put(host_string, resolved_ip.to_string());
        resolved_ip
    } else {
        let found = found.unwrap();
        debug!("resolve_with_cache: host={} found in cache, returning: {}", host_string, found);
        String::from(found)
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
    inner: JoinHandle<Result<IpAddrs, std::io::Error>>
}

pub async fn resolve_to_result(host: String,
                               resolver: Arc<TokioAsyncResolver>,
                               cache: Arc<Mutex<LruCache<String, String>>>) -> Result<IpAddrs, Error> {
    let ip = resolve_with_cache(host.as_str(), &resolver, cache).await;
    let ip_addr: IpAddr = ip.parse().unwrap();
    let sock = SocketAddr::new(ip_addr, 0);
    Ok(IpAddrs { iter: vec![sock].into_iter() })
}

impl Service<Name> for CacheResolver {
    type Response = CacheAddrs;
    type Error = std::io::Error;
    type Future = CacheFuture;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, name: Name) -> CacheFuture {
        debug!("CacheResolver.call resolving host={:?}", name.as_str());
        let resolver: Arc<TokioAsyncResolver> = self._resolver.clone();
        let cache: Arc<Mutex<LruCache<String, String>>> = self._cache.clone();
        let addrs = tokio::task::spawn(
            resolve_to_result(String::from(name.as_str()), resolver, cache)
            // resolve_with_cache(host.as_str(), &resolver, cache)
        );
        CacheFuture { inner: addrs }
    }
}

impl Future for CacheFuture {
    type Output = Result<CacheAddrs, std::io::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map(|res| match res {
            Ok(Ok(addrs)) => Ok(CacheAddrs { inner: addrs }),
            Ok(Err(err)) => Err(err),
            Err(join_err) => {
                if join_err.is_cancelled() {
                    Err(std::io::Error::new(std::io::ErrorKind::Interrupted, join_err))
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
