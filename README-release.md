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

### Mac OS X Installation
On Mac OS X, it's really easy: just run the `install.sh` included in the distribution zip file.
This will install the flex router, initialize it, and install it as a LaunchDaemon.

You can use the `uninstall.sh` script to undo everything that install has done.
Running `uninstall.sh` will unload the LaunchDaemon and remove all files that were installed.

### Linux Installation
On Linux, start by opening a Terminal window, then follow the instructions below to run `flex_init.sh`.

### Windows Installation
If you're using Windows, start by opening a Cygwin bash shell. That's what you'll use to run these commands.
Follow the instructions below to run `flex_init.sh`.

#### If you have htpasswd installed
To initialize your bubble-flexrouter, run the init script:

```shell script
flex_init.sh
```

You will be prompted to enter a master password for the flex router. Remember this password.

You can also set this password using an environment variable

```shell script
export BUBBLE_FR_PASS=some-plaintext-password
flex_init.sh
```

The above command will read the password from the `BUBBLE_FR_PASS` environment variable and will not
prompt for a password.

#### If you don't have htpasswd installed
If you don't have `htpasswd` on your system, then you will need to manually bcrypt the password.
You can do this online at https://bcrypt-generator.com/ -- ensure that "Rounds" is set to 12.

Then set the `BUBBLE_FR_PASS` environment variable to the bcrypted password and
run `flex_init.sh` with the `--bcrypt` flag:

```shell script
export BUBBLE_FR_PASS=some-bcrypted-password
flex_init.sh --bcrypt
```
 
## Connect to your Bubble
Start the Bubble app and login. On Linux, run `wg-quick up wg0` to connect.

## Running Flex Router
You'll need to run `bubble-flexrouter` as root (on Linux/MacOS) or Administrator (on Windows).

#### Set Environment
Set environment variables required to run the flex router.

These defaults should work, where `${HOME}` is the home directory of the user who ran `flex_init.sh`:

```shell script
export BUBBLE_FR_SSH_KEY=${HOME}/.ssh/flex
export BUBBLE_FR_PASS=${HOME}/.bfr_pass
export BUBBLE_FR_TOKEN=${HOME}/.bfr_token
```

On Windows, if you are using the standard Windows `cmd` program,
replace `export` with `set` and `${HOME}` with `C:\cygwin64\home\<username>`
where `<username>` is the name of the user who ran `flex_init.sh`

#### Run the router
Now that you have your environment variable set, you can run the router.

On Linux and Mac OS:

```shell script
sudo bubble-flexrouter
```

On Windows, use `runas` to run `bubble-flexrouter` as Administrator:

```shell script
runas /user:domainname\username bubble-flexrouter
```

To see all available options:

```shell script
bubble-flexrouter --help
```

## Register the flex router with your Bubble
This step can be done as a regular user (non-root, non-Administrator).

#### Register the router
Run:

```shell script
flex_register.sh your-bubble-hostname.example.com
```

Where `your-bubble-hostname.example.com` is the hostname of your Bubble.

If you're not sure what the hostname is, click on "My Bubble" in the Bubble App and copy the hostname
from your browser's location bar.

On Linux, the hostname is not easily accessible, but you can use the IP address of your Bubble just the same.
To get the IP address of your Bubble on Linux, run:

```shell script
cat /etc/wireguard/wg0.conf | grep Endpoint | awk -F':' '{print $1}' | awk '{print $NF}'
```

#### Using Environment Variable to Register
When you run `flex_register.sh`, you'll be prompted for your flex router password, your Bubble account email,
and your Bubble account password.

If you don't want to enter these every time, you can set environment variables instead.

Set the `BUBBLE_FR_PASS` environment variable to the actual plaintext password for your flex router.
This is what you used when running `flex_init.sh` to set up the router.

Set the `BUBBLE_USER` and `BUBBLE_PASS` environment variables to your Bubble account email and password.

For example:

```shell script
export BUBBLE_FR_PASS=the-password-you-used-when-running-flex_init.sh
export BUBBLE_USER=your-bubble-email@example.com
export BUBBLE_PASS=your-bubble-password
```

On Windows, replace `export` with `set` if you are using the standard Windows `cmd` program.

## Running the router
You can sit back and let the router do its work. It will periodically check to make sure that its
secure tunnel to the Bubble is OK. If it finds and problems, it will re-establish the tunnel.

On the other side, your Bubble will be monitoring the router to ensure it is available and properly functioning.

## Re-register every time your start bubble-flexrouter 
**Every time** you start the `bubble-flexrouter`, you need to register it with your Bubble using `flex_register.sh`

If you start `bubble-flexrouter` and never run `flex_register.sh`, then your Bubble will not know the router is
available and it will not be used for flex routing.
