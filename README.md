# World Host Server

This is the server software for [World Host](https://github.com/Gaming32/world-host) that manages a list of online players and communications between them. It also serves as a proxy server for when UPnP is not available.

## Proxy server

For the proxy server to work, `--base-addr` needs to be passed, and a wildcard domain needs to be set up. For example, if `wh.example.com` is passed, there needs to be a CNAME for `*.wh`.

## Analytics

Basic analytics about how many players are online as well as how many players are from each country are written to `analytics.csv` while the server is running. Information will be flushed to this file with the period specified with `--analytics-time`. Analytics are disabled by default.

## Configuring

Currently, configuration is only through command-line parameters.

```
-p, --port <PORT>                      Port to bind to [default: 9646]
-a, --base-addr <BASE_ADDR>            Base address to use for proxy connections
-j, --in-java-port <IN_JAVA_PORT>      Port to use for Java Edition proxy connections [default: 25565]
-J, --ex-java-port <EX_JAVA_PORT>      External port to use for Java Edition proxy connections
    --analytics-time <ANALYTICS_TIME>  Amount of time between analytics syncs [default: 0m]
    --shutdown-time <SHUTDOWN_TIME>    The amount of time before the server automatically shuts down. Useful for restart scripts
    --log-config <LOG_CONFIG>          The path to a log4rs yaml logging configuration
-h, --help                             Print help
-V, --version                          Print version
```
