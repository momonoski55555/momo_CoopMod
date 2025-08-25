require('myconfigs')

-- Helper for managing persistence of tables across save/load
require('helpers/tableSave')

-- Uncomment to use EOP Helper functions
-- require('helpers/EopLuaHelpers')

-- Uncomment to use external debugger
-- require('helpers/mobdebug').start()

-- Our campaign config table.
campaignConfig = { ["someConfigValue"] = 5 };

local pipe = "\\\\.\\pipe\\cruesader_pipe";

-- Fires when loading a save file
pipe_info_result = "" -- Make sure this is global
pipe_error_result = "" -- New global for detailed error

-- Require our new pipe module
local pipe_module = require('redist/pipe_module/init')

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
            "3. Pipes are local-only - no firewall needed"
    end
    
    print("\nScript finished.")
end

-- Call the function on script load
connect_to_pipe()

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

-- Fires when creating a save file
-- Returns a list of M2TWEOP save files
function onCreateSaveFile()
    local savefiles = {};
    currentPath = M2TWEOP.getPluginPath();

    -- Function from helper, save our table
    persistence.store(currentPath .. "configTable.lua", campaignConfig);

    savefiles[1] = currentPath .. "configTable.lua";
    return savefiles;
end

-- Fires when the plugin is first loaded at game start or restarted with restartLua()
function onPluginLoad()
    M2TWEOP.unlockGameConsoleCommands();
    -- UNCOMMENT TO ENABLE BELOW SETTINGS
    --M2TWEOP.setAncillariesLimit(8);
    --M2TWEOP.setMaxBgSize(31);
    --M2TWEOP.setReligionsLimit(10);
    --M2TWEOP.setBuildingChainLimit(9);
    --M2TWEOP.setGuildCooldown(3);
end


--- Called after loading the campaign map
function onCampaignMapLoaded() 
    GAME_DATA = gameDataAll.get()
    CAMPAIGN = GAME_DATA.campaignStruct
    STRAT_MAP = GAME_DATA.stratMap
    BATTLE = GAME_DATA.battleStruct
    UI_MANAGER = GAME_DATA.uiCardManager
end

---@param pDevice LPDIRECT3DDEVICE9 
function draw(pDevice)
    if (ImGui.IsKeyPressed(ImGuiKey.GraveAccent))
    and (ImGui.IsKeyDown(ImGuiKey.LeftCtrl))
    then
        M2TWEOP.toggleConsole()
    elseif (ImGui.IsKeyPressed(ImGuiKey.GraveAccent))
    and (ImGui.IsKeyDown(ImGuiKey.LeftAlt))
    then
        M2TWEOP.toggleDeveloperMode()
    elseif (ImGui.IsKeyPressed(ImGuiKey.R))
        and (ImGui.IsKeyDown(ImGuiKey.LeftCtrl))
        and (ImGui.IsKeyDown(ImGuiKey.LeftShift))
    then
        M2TWEOP.restartLua()
    end
    ImGui.Begin("debug menu",true)

    ImGui.Text("pipe path V")
    ImGui.Text(pipe)
    if ImGui.Button("Test Pipe Connection", 150, 25) then
        connect_to_pipe()
    end
    ImGui.Text("Connection Status: " .. pipe_info_result)
    ImGui.Text("Error Details: " .. pipe_error_result)

end

