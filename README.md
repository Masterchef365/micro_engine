# Implementation TODO
[ ] Transforms impl
[ ] Move some cubes with a script
[ ] Add some meshes with a script
[ ] Hotload script
[ ] Hotload shaders
[ ] Interactive prompt 
[ ] Multiple scripts/prompt switching

# LUA interface for rendering, very simple:
Functions your scripts may have:
* `init()` function, called once (returns something)
* `frame(obj)` function, called each frame (go figure)
    * Takes as argument that object that was passed last time
    * Must return an array of tuples, `(material, transform)`
* `event(event, obj)` function, called each event

Functions you can call:
* `mesh(vertices, indices)`: Takes a table of vertices and a table of indices and returns a Mesh object
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
* Do scripts run in their own namespaces? 
    * If so, which namespace is the prompt on? 
    * Do I need something like an enter/exit for prompts? 
        * It would looke like `myscript> ` when inside a script - it would also be cool to just do a global table and then individual scripts... Hmm... Or maybe you can just do it simpler - one global namespace and then a bunch of scripts

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