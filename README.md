bubble-flexrouter
=================
bubble-flexrouter provides HTTP/HTTPS proxy services for Bubble.

Some websites and apps refuse to respond to requests originating from a cloud IP address.
Thus, when a user is connected to their Bubble, some sites and apps will not work.

With flex routing, Bubble can route these requests through a device connected to the Bubble that is running bubble-flexrouter.
Now, from the perspective of the website or app, these requests will originate from a "clean" IP, and so a valid response
will be sent.

Note that using flex routing does remove some privacy protection - sites and apps that are flex-routed will see
one of your device's real IP addresses.

# The Easy Way
Download the latest bubble-flexrouter release:

  * [bubble-flexrouter for Windows](https://jenkins.bubblev.org/public/releases/bubble-flexrouter/bubble-flexrouter-windows/latest/bubble-flexrouter.zip)
  * [bubble-flexrouter for Mac OS X](https://jenkins.bubblev.org/public/releases/bubble-flexrouter/bubble-flexrouter-macos/latest/bubble-flexrouter.zip)
  * [bubble-flexrouter for Linux](https://jenkins.bubblev.org/public/releases/bubble-flexrouter/bubble-flexrouter-linux/latest/bubble-flexrouter.zip)

The [README](README-release.md) file found within the above release ZIP file describes how to use
the `flex_init.sh` and `flex_register.sh` scripts to manage a flex router.
These scripts make managing a flex router much easier than what is described here.

The instructions below describe the low-level way to initialize and register a flex router
and are intended for software developers.

# Build bubble-flexrouter
You can skip this step if you have a pre-built binary from one of the above ZIP files.

If you want to build from source:

 * Follow the [build instructions](BUILD.md) to build the bubble-flexrouter binary.
 * or, on Windows, follow the [Windows build instructions](BUILD-windows.md).

# Installation
There are a few steps to installation:
 * Generate the flex-router password
 * Create an SSH key pair
 * Create an auth token file
 * Install system service

## Generate the bubble-flexrouter password
During installation, choose a password for the service. It should be random and at least 30 characters long.

Store this password in securely someplace where the Bubble app can read it. Ideally this is *not* on the filesystem,
but in some internal app storage mechanism, since it will be stored in plaintext.

Bcrypt the password (use 12 rounds) and store the bcrypted value in a file. This file should only be readable by
the bubble-flexrouter system service.

## Create an SSH key pair
During installation, generate an RSA key pair:

    ssh-keygen -t rsa -f /some/secure/location

In the above, `/some/secure/location` should be a path that is only readable by the bubble-flexrouter system service.

When this step is done, `/some/secure/location` should be the path to the SSH private key and
`/some/secure/location.pub` should be the path to the SSH public key.

## Create an auth token file
bubble-flexrouter uses an auth token to secure its connection to a Bubble.

During installation, write a random token to a file. This token must be at least 50 characters long.
After writing the file, ensure that it is only readable by the bubble-flexrouter service.

## Install system service
Install bubble-flexrouter as a system service (Windows Service or Mac OS launch daemon) during Bubble app installation.

It should always be running. Set it to run at system startup.

The user that bubble-flexrouter runs as must have sufficient privileges to add and remove IP routes from the
system routing table. This usually means Administrator (on Windows) or root (on Mac OS).

The service requires some environment variables to be set:

 * `BUBBLE_FR_SSH_KEY` - full path to the *private* SSH key
 * `BUBBLE_FR_PASS` - full path to the bcrypted password file
 * `BUBBLE_FR_TOKEN` - full path to the auth token file

Run the service with these environment variables set.

## Uncommon configuration
By default bubble-flexrouter will listen on 127.0.0.1 on ports 9823 and 9833.

If these ports are unavailable, you can change them with command line arguments to bubble-flexrouter.

Run `bubble-flexrouter --help` to see the full list of command line options. Usually you will not need to set any arguments.


# Registering
When a user successfully logs in to a Bubble node, the API response will include a session token. Use this token
and the bubble-flexrouter password to register the flexrouter with the Bubble.

To register the flexrouter with the Bubble, send a registration request to the admin port, listening on 127.0.0.1:9833

This request must include the request header `Content-Type: application/json`

    POST http://127.0.0.1:9833/register
    {
      "password": "<password>",
      "session": "<session-token>",
      "bubble": "<bubble-hostname>",
      "ip": "<client-vpn-ip>"
    }

Where:

  * `<password>` is the bubble-flexrouter password that was generated during installation
  * `<session-token>` is the session token returned when the used logged in (usually from the `auth/login` API call)
  * `<bubble-hostname>` is the hostname of the Bubble that the app has connected to
  * `<client-vpn-ip>` is the VPN IP address that was assigned to the device (usually starts with `10.19.`)

A successful registration request will return HTTP status 200. Any other response indicates a failure, and the response
body will contain a plaintext string with an error message.

An example using curl:

    curl -H 'Content-Type: application/json' \
         -d '{"password":"Uy6dDwNP5msid3P6QEpeVmQMuUiAda","session":"47cc4974-2eca-47d8-8c74-c2cc106b9ba8","bubble":"nexus-dr66b-wn85d-ux27e.bubv.net","ip":"10.19.49.12"}' \
         http://127.0.0.1:9833/register


# Unregistering
When a user logs out of a Bubble node, unregister the flexrouter by sending a request to the admin port.

This request must include the request header `Content-Type: application/json`

    POST http://127.0.0.1:9833/unregister
    {
      "password": "<password>"
    }

Where:

  * `<password>` is the bubble-flexrouter password that was generated during installation

A successful unregister request will return HTTP status 200. Any other response indicates a failure, and the response
body will contain a plaintext string with an error message.

An example using curl:

    curl -H 'Content-Type: application/json' \
         -d '{"password":"Uy6dDwNP5msid3P6QEpeVmQMuUiAda"}' \
         http://127.0.0.1:9833/unregister


# Uninstallation
If the Bubble app is uninstalled from the system, then also:

  * Stop and remove the system service
  * Remove the bcrypted password file
  * Remove both files of the SSH key pair
  * Remove the auth token file
