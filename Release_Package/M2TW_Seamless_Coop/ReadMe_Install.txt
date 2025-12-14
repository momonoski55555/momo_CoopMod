=== Medieval II Total War: Seamless Coop Mod ===
Installation Instructions

Prerequisites:
1. Medieval II Total War installed.
2. Engine Overhaul Project (EOP) installed for your mod (e.g., Crusades).

How to "Connect":
This mod uses Dropbox to sync save files between players. 
For this to work, YOU AND YOUR FRIEND MUST USE THE *SAME* DROPBOX ACCOUNT (or the same Access Token).
You are not connecting directly to each other; you are both connecting to the same Dropbox storage.

Installation:
1. Open your Mod folder (e.g., mods/crusades).
2. Go to 'youneuoy_Data/plugins'.
3. Copy the contents of the 'Plugins' folder from this package into 'youneuoy_Data/plugins'.
   - It should look like: youneuoy_Data/plugins/lua/LuaPluginScript.lua
   - And: youneuoy_Data/plugins/lua/redist/pipe_module/...

Server Setup:
1. Go to the 'Server' folder in this package.
2. Open 'config.toml' with Notepad.
3. Paste the SHARED Dropbox Access Token where it says "INSERT_YOUR_DROPBOX_TOKEN_HERE".
   - (For the Host: Generate this token from the Dropbox Console)
   - (For the Friend: Ask the Host to send you this token)
4. Run 'Run_Server.bat'.

Playing:
1. Start the game with EOP.
2. The Server window should be open.
3. When you end your turn, the mod will automatically upload the save to the shared Dropbox folder.
4. The other player will see the update and download it automatically.
