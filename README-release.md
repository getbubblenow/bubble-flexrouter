# Bubble Flex Router
bubble-flexrouter provides HTTP/HTTPS proxy services for Bubble.

Some websites and apps refuse to respond to requests originating from a cloud IP address.
Thus, when a user is connected to their Bubble, some sites and apps will not work.

With flex routing, Bubble can route these requests through a device connected to the Bubble that is running bubble-flexrouter.
Now, from the perspective of the website or app, these requests will originate from a "clean" IP, and so a valid response
will be sent.

Note that using flex routing does remove some privacy protection - sites and apps that are flex-routed will see
one of your device's real IP addresses.

## Required software
To use the `flex_init.sh` and `flex_register.sh` tools, you'll need to have some software installed:

  * bash
  * curl
  * jq
  * htpassword

On Windows, use [Cygwin](https://cygwin.com) to install these.
`htpasswd` is not available from Cygwin. See below for a workaround.

## Overview
In order to use bubble-flexrouter, you must:

  * Initialize the flex router
  * Connect to your Bubble
  * Run bubble-flexrouter
  * Register the flex router with your Bubble

We'll walk through each of these steps next.

## Initialize the flex router
This step only needs to be done once. After that, bubble-flexrouter will re-use the initialization settings.

If you're using Windows, start by opening a Cygwin bash shell. That's what you'll use to run these commands.

### If you have htpasswd installed
To initialize your bubble-flexrouter, set the flexrouter master password and run the init script:

    export BUBBLE_FR_PASS=some-plaintext-password
    flex_init.sh

### If you don't have htpasswd installed
If you don't have `htpasswd` on your system, then you will need to manually bcrypt the password.
You can do this online at https://bcrypt-generator.com/

    export BUBBLE_FR_PASS=some-bcrypted-password
    flex_init.sh --bcrypt
 
## Connect to your Bubble
Start the Bubble app and login. On Linux, run `wg-quick up wg0` to connect.

## Running Flex Router
Then run:

    bubble-flexrouter

To see all available options:

    bubble-flexrouter --help

## Register the flex router with your Bubble
Run:

    flex_register.sh your-bubble-hostname.example.com

Where `your-bubble-hostname.example.com` is the hostname of your Bubble.

If you're not sure what the hostname is, click on "My Bubble" in the Bubble App and copy the hostname
from your browser's location bar.

On Linux, the hostname is not easily accessible, but you can use the IP address of your Bubble just the same.
To get the IP address of your Bubble on Linux, run:

    cat /etc/wireguard/wg0.conf | grep Endpoint | awk -F':' '{print $1}' | awk '{print $NF}'

## Restarting bubble-flexrouter
Every time you start bubble-flexrouter, you need to run `flex_register.sh` to register the router
with your Bubble.
