# API Stability in Development

There are three different types of API stability:

1. User-facing
2. Backend
3. Behavioral

## User-facing

User-facing API stability is the main type of API stability everyone thinks of when
they hear the words "breaking changes". It regards changes to the usage of an API when
a new version is released. For example, adding a function in version `0.5` and removing
it in version `0.12` constitutes user-facing API breakage.

When working on the APIs, strive to not change already existing functions for minor reasons like
parameter ordering and the like. If you want to make a breaking change like renaming or removing
a function, you must deprecate the function first. It can then be removed however many major
versions later as specified in the
[configuration page on API stability](../configuration/api-stability#regarding-the-user-facing-api).

## Backend

Backend API stability regards the stability of the types and protocols used to
communicate with Pinnacle over gRPC. A breaking change to the backend breaks
current versions of the API, as they assume an older protobuf schema and protocol.

To mitigate breaking changes to the protobuf schema and protocols used, protobuf files
are versioned. Each version is handled separately within the compositor. If you need to make
a breaking change to any protobuf types or the protocol used between the API and compositor,
you will need to create a new version of the protobuf file and handle that in the code.

## Behavioral

Behavioral API stability regards changes in behavior as a result of updates to how the compositor
works. For example, if we change Xwayland to require a function call to start, this silently
changes how current configs need to function. There is no user-facing or backend API breakage,
but when people update Pinnacle they will suddenly find that Xwayland no longer starts.

Try not to make any changes that require the bahavior of already existing functions to change.
This makes it hard for users to understand what has changed or is going wrong. If possible,
encapsulate the new behavior in a different function.
