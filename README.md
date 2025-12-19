<img width="2248" height="768" alt="mcmo" src="https://github.com/user-attachments/assets/5a068fe6-c790-4896-9a47-312b7f0293ad" />

**Momo's Coop Mod**: this a campaign mod for medieval 2 aimed to make it possible to play campaign online with online battles without manually transferring save file between computers. 

Discord: coming soon.

# **How to install**
1. Open your Mod folder (e.g., mods/crusades).
2. Go to 'youneuoy_Data/plugins'.
3. put the 'Plugins' folder from this package into 'youneuoy_Data/plugins'.
   - It should look like: youneuoy_Data/plugins/lua/LuaPluginScript.lua
   - And: youneuoy_Data/plugins/lua/redist/pipe_module/
Now you should have everything setup now you need to setup the dropbox here. -> https://www.dropbox.com/developers/

click create app and fill the questions and click create app

<img width="1490" height="742" alt="Pasted image 20251217181515" src="https://github.com/user-attachments/assets/5d658e4f-9bff-4879-9745-8818ec1837e0" />

answer the questions and click create app

<img width="376" height="152" alt="Pasted image 20251217181627" src="https://github.com/user-attachments/assets/010aa84c-21d5-42fc-94d7-765856207e11" />

generate the token then go into server then in config.toml and paste it here **NOTE**: Token's expire every 4 hours by the way 

<img width="1156" height="275" alt="Pasted image 20251217191325" src="https://github.com/user-attachments/assets/83a29f0b-0ee9-49eb-8344-494adca35fe5" />

# **How to use**
Run server.exe in the server folder.
and launch medieval 2 from the eop gui.exe not the from steam.

while your playing a menu like this would appear.
<img width="1079" height="712" alt="Pasted image 20251217220554" src="https://github.com/user-attachments/assets/3e4abcf6-16d2-480f-9c02-7b3cf9ab1e1f" />
After you end your turn **YOU SHOULD NAME YOUR SAVE QUICKSAVE NOT ANYTHING ELSE**.
if not the save not won't be able to be uploaded

<img width="482" height="277" alt="Pasted image 20251217223244" src="https://github.com/user-attachments/assets/10910f39-bdc1-49eb-8be7-dffe08d0305c" />

look in the console and see if it says error troubleshoot with the error code

# **How it works**
it uses m2tweop to notify a rust app to upload your saves to dropbox cloud then have other players download it.

# Troubleshooting guide
**401**: means your token is bad regenerate it.

**2** : File Not Found (pipe server not running OR pipe name mismatch).

**5** : Access Denied (check permissions).

**231** : Pipe Busy (server not accepting connections).

**109** : Broken Pipe (connection terminated).

**53** : Network Path Not Found (not applicable for local pipes).

# Disclaimer: 
**this mod is still in very early development** this mod has alot of aspects that are unfinished.

## AI
initially the project was made by hand. then i used ai when it became confusing to fix issues (thats why the code looks like ai code i had a lot of issues). i used ai to make the pipe code.

# The future
i want to stop using dropbox for other reasons mainly token's, not having enough control over everything and logic.

rewrite the pipe code ❌

full rewrite ❌

file restructure ❌

logging ❌

automatic battle/results transfers ❌

campaign conig/admin gui ❌

chat ❌

player diplomacy ❌

player analytics ❌


