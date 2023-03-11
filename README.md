# postars

Rust Email client

# Development

Before running it, you'll need to generate your own Microsoft OAuth2
credentials. See
https://docs.microsoft.com/azure/active-directory/develop/quickstart-register-app

* Register a `Web` application with a `Redirect URI` of
  `http://localhost:3003/redirect`.
* In the left menu select `Overview`. Copy the `Application (client) ID`
  as the CLIENT_ID.
* In the left menu select `Certificates & secrets` and add a new client
  secret. Copy the secret value as CLIENT_SECRET.
* In the left menu select `API permissions` and add a permission. Select
  Microsoft Graph and `Delegated permissions`. Add the `Files.Read`
  permission.

In order to run the example call:

```
sh CLIENT_ID=xxx CLIENT_SECRET=yyy cargo run
--example msgraph
```

...and follow the instructions.

