# Micro Engine
A very WIP scriptable game and visualization framework

# Implementation TODO
- [x] Transforms impl
- [x] Move some cubes with a script
- [x] Add some meshes with a script
- [x] Hotload script
- [x] Interactive prompt 
- [x] Better error handling
- [x] LightUserData for slotmap keys/handles in LUA
- [ ] Reset function
- [ ] Keyboard input
- [x] More primitives for scripts (Lines, Points, ...)
- [ ] Text
- [ ] Window dimensions
- [ ] Mouse input
- [ ] Textures
- [x] Hotload shaders
- [ ] Dynamic meshes

# LUA interface for rendering, very simple:
Functions your scripts may have:
* `reload()`: called every script load
* `frame()`: called each frame (go figure)
    * Must return an array of tables of `{ material, transform }`
* `event(event)`: called each event

Functions you can call:
* `add_mesh(vertices, indices)`: Takes a table of vertices and a table of indices and returns a Mesh object
* `shader(vertex, fragment, obj)`: Takes two strings, one for each of the sources for the shaders
    * And maybe one more table describing the other pipeline params
    * Might just be the "body" of the shader, and some properties like transform matrix are just available
    * Don't add this immediately, just do the shader source updates thing

# Interactive design
* You can access a console using the same program-space as your script any time
    * (Probably just an MPSC channel... would be interesting to preempt it too!)
* When you save your script, it will begin to be used (until an error, in which case it will halt and wait for you to update again)
* Maybe also allow saved shaders, with a setup so that their names are available in the lua main table as shaders. So you might also have an update shader command for the lua interface. Maybe not though...

# Features
* Timeout for LUA scripts and a warning/halt
* Events for hand movement/buttons in VR...
* Extra math library for LUA

# Thoughts
* How will we track shader and script updates, when dofile() and require() are the preferred methods to pull in other code? 
    * We could search the files for those commands, but the problem is that there can be misdirections (variables in dofile, consts in include probably)
    * Have just dedicated "shader" and "script" folders. 
        * This would be so that recursive mode doesn't track too many files - because whatever path you pass to `add_shader()` would be the update path
        * Maybe the same for dofile - we do an alias to the original function, but add the tracking behaviour? "Magic" - or maybe an `reload list` spamming the paths of the files that have updated _AND_ previously requested by LUA during this session. 
    * Make the scripts able to trigger reload on their own and/or an ipc mechanism to add this as an addon and allow for more elaborate innovation
* Gpu-driven rendering, configurable from Lua...
* I should provide some way of getting the camera pose from within Lua, but it should never really need to set the camera positions...? Oh wait...
    * Okay, so there will be a "Pre" transform given to each portal view.

# Moonshot ideas
* Different levels of scripting:
    1. High-level LUA interpreter
    2. Mid-level WASM acceleration plugins for use in LUA or just scripts on their own
    3. Low-level Engine code (Rust)
* Use it for your portal game for game logic/editing. It would also be poggers to be able to save some of the state...

```lua
function frame()
    return {}
end
```
