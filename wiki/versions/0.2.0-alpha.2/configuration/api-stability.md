# API Stability

Despite being in progress for a decent amount of time now, Pinnacle is still in its early days.
This means the Rust and Lua APIs *will* break at one point or another.

More information on API stability can be found over at the
[development page for the same topic](../development/api-stability).

I am currently committing to the following in terms of API stability:

## Regarding the user-facing API

> AKA your config should not break when updating the config library version

- **Best effort** for one major version

This means, for example, if a function is introduced in version `0.2.0` and is planned
for removal, it will be *deprecated* in version `0.3.0` before being removed in version
`0.4.0`. This will provide a buffer of one version to get your current config updated.

Unfortunately, not everything can be deprecated like that; we may need to change the
signature of a function, for example. Therefore we will try to prevent breaking changes
on a *best effort* basis for one major version.

## Regarding the backend

> AKA your config should not break when updating Pinnacle itself

- At least **three major versions** of stability

This means future versions of Pinnacle should still work with configs from at most
three versions prior, e.g. `pinnacle 0.7.0` should still work with `pinnacle-api 0.4.0`.

By "backend" I mean the specific types and protocols that are used
over the underlying gRPC connection the API uses to talk to Pinnacle. A breaking change
to the backend means that **your config will break when updating Pinnacle
even if you don't touch it**. Ideally we would never have to make breaking
changes to the backend, but I am making no promises in that regard because that would
force us to keep around old code, bloating the codebase.
