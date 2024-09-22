PLAYBOX TODO
============

shader reflection + simpler parameterisation

effect system

resource trashcan

transient resources

game ui
	- health/blood bar
	- interaction prompts
	- dialogue popup
	- hand

stuff to interact with

editor
	- make saving/loading a dialog, don't just always save over default

spooky segmented worm

textured walls

snap vertices to integer positions



MISC TODO
=========

particle system

preference/settings system
	- load from disk or command line
	- something like sources commands? load maps with it

flesh out texture support
	- async? like streamed audio playback?

basic text rendering/layout
	- use a crate obvs
	- dependent on texture support - maybe?
	- could use vector fonts + mesh builder interface


procedural sound sources

audio fx chains/routing
	- need to be able to set parameters from controllers

looped audio playback
	- may tie into procedural sound sources
	- need an api for play/pause

audio spatialisation
	- could fit into fx chain

sound builder interface?
	- for building sounds offline



intersection math
	- circle/box/aabb/line/ray/plane
	- line/ray-casting
	- shortcut for raycasting toy-scenes, or some preprocessed toy scene


figure out how to handle UI
	- strict MVC the way I've been doing it might be too much for each usecase
	- needs to be some pattern for tying visual elements to interactions
	- depends on intersection math probably, also text rendering/texture support


resource management
	- how to stop long running audio nodes when the owning game state ends
	- how to remove dropped actions from input context


input
	- separate binding config from runtime context/action.
		- for bindings to be useful they have to all be known ahead of time
		- contexts and actions don't necessarily exist from the beginning







botw bling effect

menu system

scene transitions

separate foot controller
	make critters

that bowls in water art installation

bugs

banjo kazooie style music note particles

rain effect
	render depth map from above
	simulate drops on gpu
	reset on depth failure

fps dungeon - orthogonal level from image
	- hand sprite in screen space
	- on screen buttons
	- 90° turns

bilboard mesh builder?

bird call synth

export geometry node stuff

rug pattern designer - decorate a little room

make sacrifices to make your crops/garden grow
sacrifice people to make your animals happy

offset screen printing effect

remake sheep

patterns
	- https://twitter.com/hyappy717/status/1442895491360952321
	- https://github.com/SYM380/p5.pattern

noise dither gradients

flow fields and curves
	- https://tylerxhobbs.com/essays/2020/flow-fields

split Node trait into SourceNode, EffectNode and co to allow some nodes to process in place
	- can improve memory usage

a landline phone that has to be unplugged to make calls



resource cleanup?
	- resource contexts?
	- resource scope?
	- move resource creation out of gfx::Context into resources subobject

builtin default shaders for provided vertex types

future based helper for writing 'sketches'
```rust
async {
	// resource set up

	loop {
		let engine = next_frame().await;

		// update and render
	}
}
```
