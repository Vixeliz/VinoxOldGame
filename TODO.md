* Refactor code base to be more modular and smaller bits.
* Copy chunk edges into other chunk edges for meshing data only.
* Proper collisions either stay with rapier or switch to custom aabb solution
* Refactor networking code to make more sense for example for more enemies in the future and events and such. Also make helper
functions for stuff like creating chunk_data to send instead of in the receive function
* Custom skins
* Basic scripts for blocks and entities
* Entities
* GUI
* Simple chat
* Inventory
* Modifying the world
* Interacting with world and entities
* Sending chunks around the player
* Save/Load(we are gonna use an sqlite database instead of making a custom format similiar to minetest or minecraft bedrock(except bedrock uses leveldb))
* Singleplayer(will just be multiplayer but a locally ran server)
* Health
* Basic combat
* Crafting(will be like terraria where you just need to have a certain amount of materials for a given recipe instead of grid)
* Tameable entities
* Hostile entities
* AI for entities
* lighting baked at mesh building
* particles
* items(will be defined similiar to blocks and entities. However blocks will automatically make a associated item)
* Options such as controls and graphics options
* Better texture atlas coord getter thing
* Visibility for voxels
* Loading custom models for specified voxels and setting the visibility to empty for meshing
* Liquid
* Farming
* Some type of simple logic system in game(something like redstone but not)
* A more advanced logic system in game that allows for some task to be automated using lua. For more advanced players and late game
* Usernames and possibly passwords on a per server basis (Not hosting any authentication service or anything like that)
* Biomes(will use ron files to define biome conditions such as temperature and humiditiy if multiple biomes match my idea is to have yet another noise map then each biome matching the temp and humidity exactly will be assigned a random number in a range then whichever biome is closest to the noise will be chosen)
* Caves and overhangs and such
* Structures(anything placed in the world such as trees or large boulders)
* Post processing(Underwater effect, vignette, etc)
* Multiple sides for textures
* Basic framework for simple guis for blocks that need them. Allow for stuff like inputs and output slots and acceptable items and not. By basic I mean itll use ron files as its easy to do. Scripts attached to the block will be able to process inputs and outputs
* Item tags to tag items with a group for example weapons or food
* Work on subtle plot points in game
* (LONG TERM) more advanced scripting api that gives access to more of engine features
* Multiple sub-worlds? Ie worlds that may look different that can be traveled to without leaving a server/world
* Save entities in chunks that should be saved(Those named or in a specific area marked with something)
* Connect multiple storage types together to one unified interface?
* Distance fog
* liquid physics
* equipment
* (LONG TERM) possibly switch to a less naive networking system using client prediction and such
* Simple measures against cheating most likely easily by passed but will be added such as checking to make sure player position didnt change too much
* Different tool types
* More block, entity, and item criteria in the ron files(Ie what should the mob target etc)
* This relates to entities but I want to point it out specifically is minions. Hopefully the entitiy solution will be generic enough to allow these through ron files
* Third person modes
* A preview of player in the corner
* Items should also be able to hold data similiar to blocks
* Make a more advanced block_state solution basically just have to figure out parsing and the format for blocks data. For example what direction its facing or status of an item doing something
* Fixed timestep for certain things such as checking if a crop grows.
* Whitelist and blacklist for usernames
* After all that make sure to refactor and optimize 
