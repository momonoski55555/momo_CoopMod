--- This script provides functionality for connecting to a named pipe,
--- managing campaign configuration persistence, and handling various game events
--- within the Medieval II Total War Engine Overhaul Project (M2TWEOP) plugin environment.

-- Requires `myconfigs.lua` for additional configurations specific to this setup.
require('myconfigs')

--- Helper for managing persistence of tables across save/load operations.
--- This module allows saving and loading Lua tables to/from files,
--- ensuring game state or configurations persist across game sessions.
require('helpers/tableSave')

-- Uncomment the following line to enable EOP Helper functions, if needed.
-- require('helpers/EopLuaHelpers')

-- Uncomment the following line to enable an external debugger for this script.
-- This can be useful for advanced debugging of Lua code.
-- require('helpers/mobdebug').start()

--- Our main campaign configuration table.
--- This table stores various settings and data relevant to the campaign.
campaignConfig = { ["someConfigValue"] = 5 };

--- Defines the name of the named pipe that the client will attempt to connect to.
--- This must match the name used by the pipe server (e.g., the Rust server).
local pipe = "\\\\.\\pipe\\cruesader_pipe";

--- Global variable to store the result of the pipe connection for UI display.
--- Initialized to an empty string and updated after `connect_to_pipe` is called.
pipe_info_result = ""

--- Global variable to store detailed error messages from pipe connection attempts.
--- This provides more context for troubleshooting connection failures.
pipe_error_result = ""

--- Requires the custom pipe module, which provides the `connect` function
--- for simplified interaction with named pipes.
local pipe_module = require('redist/pipe_module/init')

--- Attempts to establish a connection to the named pipe server and exchange messages.
--- It uses the `pipe_module.connect` function to abstract the pipe communication.
---
--- Prints connection status and received messages to the console.
--- Updates global variables `pipe_info_result` and `pipe_error_result` based on the outcome.
local function connect_to_pipe()
    local pipe_name = "\\\\.\\pipe\\cruesader_pipe"
    local message_to_send = "Hello from Lua client (via module)!"
    
    print("Attempting to connect to pipe: " .. pipe_name)
    
    local connection_result = pipe_module.connect(pipe_name, message_to_send)
    
    if connection_result.success then
        print("✓ Connection successful!")
        print("✓ Received from server: '" .. connection_result.received_message .. "'")
        print("✓ Sent message: '" .. message_to_send .. "'")
        pipe_info_result = "Success"
        pipe_error_result = "" -- Clear error on success
    else
        print("✗ Connection failed!")
        print("✗ " .. connection_result.full_error)
        pipe_info_result = "Connection Failed"
        pipe_error_result = connection_result.full_error .. "\n\nNamed Pipe Troubleshooting:\n" ..
            "1. Verify pipe server is running\n" ..
            "2. Confirm pipe name exactly matches: " .. pipe_name .. "\n" ..
            "3. Pipes are local-only - no firewall needed" --> Pipes are local to THIS machine/VM (not external over network)
    end
    
    print("\nScript finished.")
end

-- Immediately calls the `connect_to_pipe` function when the script is loaded.
connect_to_pipe()

--- Function called when a save file is loaded.
--- It iterates through the provided `paths` to find and load the `campaignConfig`
--- from a `configTable.lua` file, using the `persistence.load` helper function.
---
--- @param paths table A list of paths to files contained within the save.
function onLoadSaveFile(paths)
    campaignPopup = true;

    for index, path in pairs(paths) do
        if (string.find(path, "configTable.lua"))
        then
            -- Function from helper, load saved table
            campaignConfig = persistence.load(path);
        end
    end
end

--- Function called when creating a save file.
--- It saves the `campaignConfig` table to a `configTable.lua` file
--- within the plugin's save path using `persistence.store`.
---
--- @returns table A list containing the path to the saved configuration file,
---                which M2TWEOP will include in the save archive.
function onCreateSaveFile()
    local savefiles = {};
    currentPath = M2TWEOP.getPluginPath();

    -- Function from helper, save our table
    persistence.store(currentPath .. "configTable.lua", campaignConfig);

    savefiles[1] = currentPath .. "configTable.lua";
    return savefiles;
end

--- Function triggered when the plugin is first loaded at game start
--- or when it's restarted using `M2TWEOP.restartLua()`.
---
--- It unlocks game console commands and provides commented-out examples
--- of how to set various game limits or parameters using M2TWEOP functions.
function onPluginLoad()
    M2TWEOP.unlockGameConsoleCommands();
    -- UNCOMMENT TO ENABLE BELOW SETTINGS FOR CUSTOM GAME MECHANICS OR LIMITS:
    --M2TWEOP.setAncillariesLimit(8);    -- Set the maximum number of ancillaries a character can have.
    --M2TWEOP.setMaxBgSize(31);          -- Set the maximum battle map size.
    --M2TWEOP.setReligionsLimit(10);     -- Set the maximum number of religions allowed.
    --M2TWEOP.setBuildingChainLimit(9);  -- Set the maximum length of a building chain.
    --M2TWEOP.setGuildCooldown(3);       -- Set the cooldown (in turns) before a guild can offer another mission.
end


--- Called automatically by the M2TWEOP plugin after the campaign map has finished loading.
--- This function populates global variables (`GAME_DATA`, `CAMPAIGN`, `STRAT_MAP`, etc.)
--- with crucial game state information, making it accessible throughout the script.
function onCampaignMapLoaded()
    GAME_DATA = gameDataAll.get()      -- Retrieves all available game data.
    CAMPAIGN = GAME_DATA.campaignStruct -- Accesses campaign-specific structures.
    STRAT_MAP = GAME_DATA.stratMap      -- Accesses strategic map data.
    BATTLE = GAME_DATA.battleStruct     -- Accesses battle-specific data.
    UI_MANAGER = GAME_DATA.uiCardManager -- Accesses the UI card manager.
end

--- Function called every frame for drawing to the screen, typically used with ImGui.
--- This function handles key presses for toggling console, developer mode, and restarting Lua scripts.
--- It also renders a debug menu using ImGui, displaying pipe information and a connection test button.
---
--- @param pDevice LPDIRECT3DDEVICE9 The Direct3D device pointer, typically used for rendering operations.
function draw(pDevice)
    -- Check for Ctrl + Grave Accent (tilde) to toggle the game console.
    if (ImGui.IsKeyPressed(ImGuiKey.GraveAccent))
    and (ImGui.IsKeyDown(ImGuiKey.LeftCtrl))
    then
        M2TWEOP.toggleConsole()
    -- Check for Alt + Grave Accent (tilde) to toggle developer mode.
    elseif (ImGui.IsKeyPressed(ImGuiKey.GraveAccent))
    and (ImGui.IsKeyDown(ImGuiKey.LeftAlt))
    then
        M2TWEOP.toggleDeveloperMode()
    -- Check for Ctrl + Shift + R to restart the Lua scripts.
    elseif (ImGui.IsKeyPressed(ImGuiKey.R))
        and (ImGui.IsKeyDown(ImGuiKey.LeftCtrl))
        and (ImGui.IsKeyDown(ImGuiKey.LeftShift))
    then
        M2TWEOP.restartLua()
    end
    
    -- Begin rendering the "debug menu" window, allowing it to be closed by the user.
    ImGui.Begin("debug menu",true)

    ImGui.Text("Pipe Path:")
    ImGui.Text(pipe)
    -- Button to manually test the pipe connection again.
    if ImGui.Button("Test Pipe Connection", 150, 25) then
        connect_to_pipe()
    end
    -- Display the current connection status and any detailed error messages.
    ImGui.Text("Connection Status: " .. pipe_info_result)
    ImGui.Text("Error Details: " .. pipe_error_result)

    ImGui.End() -- End the ImGui window.
end

