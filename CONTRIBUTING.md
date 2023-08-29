# Repository Guidelines

Interested in contributing? Have a feature request, bug report, or question? Well, you've come to the right place!

You can use the table of contents below to navigate directly to what is relevant for you.

> ### Table of Contents
> 1. [Contributing](#contributing)
> 2. [Feature requests](#feature-requests)
> 3. [Bug reports](#bug-reports)
> 4. [Questions](#questions)

## Contributing
Thank you for taking your time to contribute! As with many repositories out there,
there are some guidelines on what you should and shouldn't do, listed below.

> ### Before you continue...
> - If you are thinking about contributing towards a new feature request that has no prior issue,
> consider opening one to gauge interest and feedback. This will help prevent unnecessary work if
> your pull request would be denied.
> - Check to see if someone else isn't already drafting a pull request on the same thing.

### By contributing to Pinnacle, you acknowledge that you have rights to the code you provide, and you agree to license your work under the GNU General Public License v3.0.

### 1. Fork the project.

### 2. Add your contributions.
- If you are contributing to the Lua API, **document user-facing functions** using Lua LS
[annotations](https://github.com/LuaLS/lua-language-server/wiki/Annotations). Provide parameter and return types,
as well as documentation that follows the following format:
    > ````lua
    > ---A short one line summary of the function.
    > ---
    > ---Additional paragraph or paragraphs detailing more information about the function
    > ---should be added below the summary.
    > ---
    > ---### Example(s)
    > ---```lua
    > ----- Provide an example/examples of the usage of the function where appropriate,
    > ----- including function overloads when applicable.
    > ---print("hi mom!") -- Print with a string
    > ---print(true) -- Print with a boolean
    > ---```
    > ---@param (if applicable)
    > ---@return (if applicable)
    > ---@see (if applicable)
    > ---@and_other_appropriate_annotations
    > function thingy(param1, param2, ...) end
    > ````
- Format Rust code using [rustfmt](https://github.com/rust-lang/rustfmt) and the provided
[rustfmt.toml](rustfmt.toml) file.
- Format Lua code with [StyLua](https://github.com/JohnnyMorganz/StyLua) and the provided
[stylua.toml](api/lua/stylua.toml) file.

> #### Keep commit messages short.
> They should also use imperative speech, e.g. "Add feature" or "Fix bug" instead of
"Added feature" or "Fixing bug".

### 3. Open a pull request against the main branch.
- Have a clear and succinct title.
- Provide a description of what the pull request is for.
- Link any issues that may be closed after merging, if any.
> If we deem the pull request unnecessary or out of the project's scope, we'll close it.

### 4. Respond to any questions and code reviews.

### 5. Wait for merge! :tada:

## Feature Requests
If you have an idea for a new feature or enhancement, follow the steps below.

> ### Before you continue...
> - Search open issues to see if someone has already requested your feature.
>     - Duplicate requests will be closed as duplicate.
> - See if your feature request can already be implemented through Lua.
>     - Pinnacle is built to give you great control and configurability. This means there *might* be a
>     way to whip up something to fulfill what you want.
>     - For example, if you want something like a scratchpad, you can rig up a keybind to toggle a window
>     in the scratchpad tag and one to toggle the scratchpad itself. TODO: show an example config for this.
>     - If you believe that what you want *can* be implemented through configuration but are unsure on
>     how to do it, search for or open a GitHub discussion! There *should* be someone around to help.

### 1. Have a clear and succinct title.

### 2. Describe your feature request or enhancement in detail.
If you have any ideas on implementation details and whatnot, include those as well. We'd love to hear your ideas!

### 3. Add appropriate labels.
All feature requests should have the https://github.com/Ottatop/pinnacle/labels/enhancement label.
If there are other labels you feel are appropriate, add them as well!

### 4. Submit the feature request!
We'll determine if what you're asking for is within the scope of and appropriate for the project.

If we're onboard, we'll start working on it, time permitting.

If we decide that your feature request isn't right for the project, it will be closed as not planned.

## Bug Reports
So... you've finally run into the dreaded `thread 'main' panicked at 'already borrowed: BorrowMutError'`.
Or maybe you haven't. Either way, if you've run into a bug, crash, or something that you don't
think should be happening, these are the guidelines you should follow when submitting a bug report.

> ### Before you continue...
> - Search open issues to see if the problem has already been reported.
>     - If you submit a bug that already has an open issue, it will be closed as duplicate.
> - Search the wiki to see if there is a solution or workaround to your problem.
> - Ensure that the problem is reproducible or at least happens more than once.
>     - See if you can reliably reproduce the problem. If you can't, but the bug happens multiple times
>     and you have a hunch on what is causing it, still submit a bug report anyway. We'll see what we can do!

### 1. Have a clear and succinct title.
You have the entire body of the issue to go into more depth, so keep the title short, sweet, and easily parsable!

### 2. Provide necessary details.
This includes the following:
- Your Linux distribution
- The version of Rust you used to compile and run Pinnacle
- Any tracebacks or logs
    - Tracebacks can be obtained by running Pinnacle with the environment
    variable `RUST_BACKTRACE=1` or `RUST_BACKTRACE=full`.
- Your Lua config (or any applicable parts)

> #### Important:
> **If you have a log, config, or similar that is over 50 lines, please either upload it to
> a place like [pastebin](https://pastebin.com/) and link to it, attach a file, or place the text in the
> `<details>` tag, as shown below. The whitespace lines and indentation are important.**
> This helps both desktop and mobile users not have to scroll several miles to reach the next comment.
> > ````md
> > <details>
> >
> > <summary>The stack trace</summary>      // Optional summary
> > 
> > ```
> > Stack trace starting at the dingaling function...
> > Many lines here...
> > Stack trace ending at main or whatever
> > ```
> > 
> > </details>
> > ````
> This will become:
> <details>
> 
> <summary>The stack trace</summary>
> 
> ```
> Stack trace starting at the dingaling function...
> Many lines here...
> Stack trace ending at main or whatever
> ```
> </details>

### 3. Go in detail regarding the bug and reproduction steps.
Document what the bug is and, if you have them, list all steps to reproduce the bug in detail.
If not, describe what you were doing when the bug happened.

### 4. Add appropriate labels.
The one label all bug reports should have is the aptly named https://github.com/Ottatop/pinnacle/labels/bug label.
If there are other labels you feel are appropriate, like https://github.com/Ottatop/pinnacle/labels/xwayland
for XWayland issues, add them as well. These labels help us filter out issues reliably.

### 5. Smash that `Submit new issue` button!
We'll get to work on it soon (hopefully).

## Questions
Have a question about the future of the project? Perhaps you're writing your own
compositor with Smithay and you need help with *that one issue* that you've been
mulling over for days.

In any case, instead of using GitHub issues, please use
[GitHub discussions](https://github.com/Ottatop/pinnacle/discussions), which I feel is
better tailored towards questions and general, well, *discussions*. GitHub issues
should only be used for bug reports and feature requests.
