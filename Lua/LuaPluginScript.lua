require('myconfigs')
require('helpers/tableSave')

-- Configuration
local pipe_path = "\\\\.\\pipe\\coop_pipe"
local pipe_module = require('redist/pipe_module/init')

-- State Variables
campaignConfig = { ["someConfigValue"] = 5 };
pipe_info_result = "Ready"
pipe_error_result = ""
local campaign = false
local turn_number = 0
local target_turn_download = 0 -- For the UI input

-- // --- PIPE HELPER FUNCTIONS --- // --

--- Sends a message to the Rust server and returns the response
local function send_pipe_command(command)
    print("[Lua] Connecting to pipe: " .. pipe_path)
    print("[Lua] Sending: " .. command)
    
    local result = pipe_module.connect(pipe_path, command)
    
    if result.success then
        print("[Lua] Success. Received: '" .. result.received_message .. "'")
        pipe_info_result = "Last Op: Success"
        pipe_error_result = ""
        return result.received_message
    else
        print("[Lua] Failed: " .. result.full_error)
        pipe_info_result = "Last Op: Failed"
        pipe_error_result = result.full_error
        return nil
    end
end

-- // --- GAME LOGIC FUNCTIONS --- // --

function Savegame_local_quicksave()
    -- Rust server expects "quicksave.sav" in the specific directory.
    -- Ensure M2TWEOP saves to the correct 'm2tweop_temp' folder or wherever Rust is looking.
    -- M2TWEOP usually saves relative to the mod folder.
    local save_name = "quicksave" 

    if turn_number > 1 then
    print("[Lua] Saving game locally as: " .. save_name)
    M2TWEOP.saveGame(save_name) 
    else
    M2TWEOP.saveGame()    
    end
    
    

end

function handle_turn_upload()
    -- 1. Save the game locally first
    Savegame_local_quicksave()

    -- 2. Tell Rust to rename and upload this save
    -- Send format: "UPLOAD:5"
    local command = "UPLOAD:" .. tostring(turn_number)
    local response = send_pipe_command(command)

    if response == "UPLOAD_OK" then
        print("[Lua] Turn " .. turn_number .. " upload confirmed.")
    else
        print("[Lua] Server reported upload issue." .. response .. pipe_error_result)
    end
end

function handle_turn_download(target_turn)
    local command = "DOWNLOAD:" .. tostring(target_turn)
    local response = send_pipe_command(command)

    if response and string.sub(response, 1, 5) == "LOAD:" then
        print("save file loaded")
        else
        print("file not requested")
    end

end

-- // --- CALLBACKS --- // --

function onFactionTurnEnd() 
    turn_number = turn_number + 1
    print("[Lua] Turn End. New Turn: " .. turn_number)
    handle_turn_upload()
end

function onCampaignMapLoaded()
    campaign = true
    
    -- Initialize Game Data pointers
    GAME_DATA = gameDataAll.get()      
    CAMPAIGN = GAME_DATA.campaignStruct 
    STRAT_MAP = GAME_DATA.stratMap      
    BATTLE = GAME_DATA.battleStruct     
    UI_MANAGER = GAME_DATA.uiCardManager
    
    -- Attempt to get current turn number from game engine if possible
    -- turn_number = CAMPAIGN.turnNumber or 0 -- (Hypothetical, depends on API)
end

function onUnloadCampaign()
    campaign = false
end

-- // --- UI & RENDERING --- // --

function draw(pDevice)
    -- Hotkeys
    if (ImGui.IsKeyPressed(ImGuiKey.GraveAccent) and ImGui.IsKeyDown(ImGuiKey.LeftCtrl)) then
        M2TWEOP.toggleConsole()
    elseif (ImGui.IsKeyPressed(ImGuiKey.GraveAccent) and ImGui.IsKeyDown(ImGuiKey.LeftAlt)) then
        M2TWEOP.toggleDeveloperMode()
    elseif (ImGui.IsKeyPressed(ImGuiKey.R) and ImGui.IsKeyDown(ImGuiKey.LeftCtrl) and ImGui.IsKeyDown(ImGuiKey.LeftShift)) then
        M2TWEOP.restartLua()
    end
    
    -- Debug / Control Menu
    ImGui.Begin("Debug Menu", true)

    ImGui.Text("Current Local Turn: " .. tostring(turn_number))
    ImGui.Separator()

    -- UPLOAD SECTION
    if ImGui.Button("Force Upload Current Turn", 200, 25) then
        handle_turn_upload()
    end

    ImGui.Separator()

    -- DOWNLOAD SECTION
    ImGui.Text("Download Turn:")
    
    -- FIX: ImGui.InputInt in this binding likely returns the value directly, not (changed, value)
    -- This prevents target_turn_download from becoming nil and causing a crash on the next frame.
    target_turn_download = ImGui.InputInt("##download_turn", target_turn_download)
    
    if ImGui.Button("Download & Load Save", 200, 25) then
        handle_turn_download(target_turn_download)
    end

    ImGui.Separator()
    
    -- STATUS SECTION
    ImGui.Text("Pipe Status: " .. pipe_info_result)
    if pipe_error_result ~= "" then
        ImGui.TextColored(1, 0, 0, 1, "Error: " .. pipe_error_result)
    end

    ImGui.End()
end

-- // --- STANDARD CONFIGURATION CALLBACKS --- // --

function onLoadSaveFile(paths)
    for index, path in pairs(paths) do
        if (string.find(path, "configTable.lua")) then
            campaignConfig = persistence.load(path);
        end
    end
end

function onCreateSaveFile()
    local savefiles = {};
    currentPath = M2TWEOP.getPluginPath();
    persistence.store(currentPath .. "configTable.lua", campaignConfig);
    savefiles[1] = currentPath .. "configTable.lua";
    return savefiles;
end

function onPluginLoad()
    M2TWEOP.unlockGameConsoleCommands();
end