// Tauri API imports
const { invoke } = window.__TAURI__.tauri;
const { appWindow } = window.__TAURI__.window;

// State management
let currentConfig = {};
let isDirty = false;
let recordingHotkey = null;

// Initialize
document.addEventListener('DOMContentLoaded', async () => {
    setupEventListeners();
    await loadConfig();
    setupValidation();
});

// Event listeners
function setupEventListeners() {
    // Tab navigation
    document.querySelectorAll('.tab-btn').forEach(btn => {
        btn.addEventListener('click', () => switchTab(btn.dataset.tab));
    });

    // Form controls
    document.getElementById('close-btn').addEventListener('click', closeWindow);
    document.getElementById('save-btn').addEventListener('click', saveConfig);
    document.getElementById('cancel-btn').addEventListener('click', closeWindow);
    document.getElementById('reset-btn').addEventListener('click', resetToDefaults);

    // API key visibility toggle
    document.getElementById('toggle-api-key').addEventListener('click', toggleApiKeyVisibility);

    // API test
    document.getElementById('test-api').addEventListener('click', testApiConnection);

    // Range inputs
    document.querySelectorAll('input[type="range"]').forEach(input => {
        input.addEventListener('input', (e) => {
            const valueSpan = e.target.parentElement.querySelector('.range-value');
            if (valueSpan) {
                valueSpan.textContent = e.target.value;
            }
            markDirty();
        });
    });

    // Hotkey recording
    document.querySelectorAll('.record-hotkey').forEach(btn => {
        btn.addEventListener('click', () => startHotkeyRecording(btn.dataset.target));
    });

    // Add alternative hotkey
    document.getElementById('add-alternative').addEventListener('click', addAlternativeHotkey);

    // Add custom prompt
    document.getElementById('add-custom-prompt').addEventListener('click', addCustomPrompt);

    // Form change detection
    document.getElementById('settings-form').addEventListener('change', markDirty);
    document.getElementById('settings-form').addEventListener('input', markDirty);

    // Prevent form submission
    document.getElementById('settings-form').addEventListener('submit', (e) => {
        e.preventDefault();
    });

    // Window close warning
    window.addEventListener('beforeunload', (e) => {
        if (isDirty) {
            e.preventDefault();
            e.returnValue = '';
        }
    });
}

// Tab switching
function switchTab(tabName) {
    // Update tab buttons
    document.querySelectorAll('.tab-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.tab === tabName);
    });

    // Update tab content
    document.querySelectorAll('.tab-content').forEach(content => {
        content.classList.toggle('active', content.id === `${tabName}-tab`);
    });
}

// Configuration loading
async function loadConfig() {
    try {
        currentConfig = await invoke('get_config');
        populateForm(currentConfig);
        isDirty = false;
    } catch (error) {
        showError('Failed to load configuration: ' + error);
    }
}

// Populate form with config data
function populateForm(config) {
    // General settings
    document.getElementById('auto-start').checked = config.general?.autoStart || false;
    document.getElementById('show-notifications').checked = config.general?.showNotifications || false;
    document.getElementById('preserve-clipboard').checked = config.behavior?.preserveClipboard || false;
    document.getElementById('auto-split-text').checked = config.behavior?.autoSplitLongText || false;

    // API settings
    document.getElementById('api-key').value = config.api?.apiKey || '';
    document.getElementById('api-endpoint').value = config.api?.endpoint || '';
    document.getElementById('model').value = config.api?.model || 'gpt-4o-mini';
    document.getElementById('temperature').value = config.api?.temperature || 0.3;
    document.querySelector('#temperature').parentElement.querySelector('.range-value').textContent = 
        config.api?.temperature || 0.3;

    // Hotkeys
    document.getElementById('translate-hotkey').value = config.hotkey?.translate || '';
    document.getElementById('cancel-hotkey').value = config.hotkey?.cancel || '';
    
    // Alternative hotkeys
    if (config.hotkey?.alternatives) {
        config.hotkey.alternatives.forEach(hotkey => {
            addAlternativeHotkey(hotkey);
        });
    }

    // Prompts
    document.getElementById('active-prompt').value = config.prompt?.activePreset || 'general';
    loadPrompts(config.prompt);

    // Limits
    document.getElementById('max-text-length').value = config.limits?.maxTextLength || 5000;
    document.getElementById('requests-per-minute').value = config.limits?.requestsPerMinute || 30;
    document.getElementById('requests-per-day').value = config.limits?.requestsPerDay || 500;

    // Update usage stats
    updateUsageStats();
}

// Load prompts into editor
function loadPrompts(promptConfig) {
    const promptList = document.getElementById('prompt-list');
    promptList.innerHTML = '';

    // Add preset prompts
    if (promptConfig?.presets) {
        Object.entries(promptConfig.presets).forEach(([id, preset]) => {
            addPromptToList(id, preset, false);
        });
    }

    // Add custom prompts
    if (promptConfig?.custom) {
        Object.entries(promptConfig.custom).forEach(([id, preset]) => {
            addPromptToList(id, preset, true);
        });
    }
}

// Add prompt to list
function addPromptToList(id, preset, isCustom) {
    const promptList = document.getElementById('prompt-list');
    const promptItem = document.createElement('div');
    promptItem.className = 'prompt-item';
    promptItem.innerHTML = `
        <h4>${preset.name} ${isCustom ? '(Custom)' : ''}</h4>
        <textarea data-prompt-id="${id}" ${!isCustom ? 'readonly' : ''}>${preset.system}</textarea>
        ${isCustom ? `<button type="button" class="btn-danger" onclick="removePrompt('${id}')">Remove</button>` : ''}
    `;
    promptList.appendChild(promptItem);

    if (isCustom) {
        promptItem.querySelector('textarea').addEventListener('input', markDirty);
    }
}

// Validation setup
function setupValidation() {
    // API key validation
    document.getElementById('api-key').addEventListener('blur', validateApiKey);

    // URL validation
    document.getElementById('api-endpoint').addEventListener('blur', validateUrl);

    // Number validations
    document.getElementById('max-text-length').addEventListener('blur', validateNumberRange);
    document.getElementById('requests-per-minute').addEventListener('blur', validateNumberRange);
    document.getElementById('requests-per-day').addEventListener('blur', validateNumberRange);
}

// Validation functions
function validateApiKey(e) {
    const input = e.target || document.getElementById('api-key');
    const value = input.value.trim();
    
    if (!value) {
        showFieldError(input, 'API key is required');
        return false;
    }
    
    if (!value.startsWith('sk-')) {
        showFieldError(input, 'Invalid API key format');
        return false;
    }
    
    clearFieldError(input);
    return true;
}

function validateUrl(e) {
    const input = e.target || document.getElementById('api-endpoint');
    const value = input.value.trim();
    
    try {
        new URL(value);
        clearFieldError(input);
        return true;
    } catch {
        showFieldError(input, 'Invalid URL');
        return false;
    }
}

function validateNumberRange(e) {
    const input = e.target;
    const value = parseInt(input.value);
    const min = parseInt(input.min);
    const max = parseInt(input.max);
    
    if (isNaN(value) || value < min || value > max) {
        showFieldError(input, `Value must be between ${min} and ${max}`);
        return false;
    }
    
    clearFieldError(input);
    return true;
}

// Field error display
function showFieldError(input, message) {
    clearFieldError(input);
    const error = document.createElement('div');
    error.className = 'field-error';
    error.textContent = message;
    error.style.color = 'var(--error)';
    error.style.fontSize = '12px';
    error.style.marginTop = '4px';
    input.parentElement.appendChild(error);
}

function clearFieldError(input) {
    const error = input.parentElement.querySelector('.field-error');
    if (error) {
        error.remove();
    }
}

// API key visibility toggle
function toggleApiKeyVisibility() {
    const input = document.getElementById('api-key');
    const button = document.getElementById('toggle-api-key');
    
    if (input.type === 'password') {
        input.type = 'text';
        button.textContent = '🙈';
    } else {
        input.type = 'password';
        button.textContent = '👁';
    }
}

// Test API connection
async function testApiConnection() {
    const apiKey = document.getElementById('api-key').value;
    const endpoint = document.getElementById('api-endpoint').value;
    const model = document.getElementById('model').value;
    
    if (!validateApiKey() || !validateUrl()) {
        return;
    }
    
    const button = document.getElementById('test-api');
    const result = document.getElementById('api-test-result');
    
    button.disabled = true;
    button.textContent = 'Testing...';
    result.className = 'test-result';
    
    try {
        const response = await invoke('test_api_connection', {
            apiKey,
            endpoint,
            model
        });
        
        result.className = 'test-result success';
        result.textContent = '✓ Connection successful!';
    } catch (error) {
        result.className = 'test-result error';
        result.textContent = '✗ ' + error;
    } finally {
        button.disabled = false;
        button.textContent = 'Test Connection';
    }
}

// Hotkey recording
function startHotkeyRecording(targetId) {
    const input = document.getElementById(targetId);
    const button = input.parentElement.querySelector('.record-hotkey');
    
    if (recordingHotkey) {
        stopHotkeyRecording();
        return;
    }
    
    recordingHotkey = targetId;
    input.value = 'Press keys...';
    button.textContent = 'Stop';
    
    // Set up key listener
    document.addEventListener('keydown', recordHotkey);
}

function stopHotkeyRecording() {
    if (!recordingHotkey) return;
    
    const input = document.getElementById(recordingHotkey);
    const button = input.parentElement.querySelector('.record-hotkey');
    
    button.textContent = 'Record';
    document.removeEventListener('keydown', recordHotkey);
    recordingHotkey = null;
}

function recordHotkey(e) {
    e.preventDefault();
    
    const modifiers = [];
    if (e.ctrlKey) modifiers.push('Ctrl');
    if (e.altKey) modifiers.push('Alt');
    if (e.shiftKey) modifiers.push('Shift');
    if (e.metaKey) modifiers.push('Meta');
    
    // Get the key
    let key = e.key;
    if (key === ' ') key = 'Space';
    else if (key === 'Escape') key = 'Esc';
    else if (key.length === 1) key = key.toUpperCase();
    
    // Combine
    const hotkey = [...modifiers, key].join('+');
    
    const input = document.getElementById(recordingHotkey);
    input.value = hotkey;
    
    // Check for conflicts
    checkHotkeyConflict(recordingHotkey, hotkey);
    
    stopHotkeyRecording();
    markDirty();
}

// Check hotkey conflicts
async function checkHotkeyConflict(inputId, hotkey) {
    try {
        const conflict = await invoke('check_hotkey_conflict', { hotkey });
        const warningDiv = document.getElementById(inputId + '-conflict');
        
        if (conflict) {
            warningDiv.textContent = `⚠ Conflicts with: ${conflict}`;
            warningDiv.classList.add('show');
        } else {
            warningDiv.classList.remove('show');
        }
    } catch (error) {
        console.error('Failed to check hotkey conflict:', error);
    }
}

// Alternative hotkeys
function addAlternativeHotkey(hotkey = '') {
    const container = document.getElementById('alternative-hotkeys');
    const id = 'alt-hotkey-' + Date.now();
    
    const div = document.createElement('div');
    div.className = 'form-group';
    div.innerHTML = `
        <div class="hotkey-input-container">
            <input type="text" id="${id}" value="${hotkey}" placeholder="Press keys..." readonly>
            <button type="button" class="btn-secondary record-hotkey" data-target="${id}">Record</button>
            <button type="button" class="btn-danger" onclick="removeAlternativeHotkey(this)">Remove</button>
        </div>
        <div id="${id}-conflict" class="conflict-warning"></div>
    `;
    
    container.appendChild(div);
    
    // Add event listener to new button
    div.querySelector('.record-hotkey').addEventListener('click', () => {
        startHotkeyRecording(id);
    });
    
    markDirty();
}

function removeAlternativeHotkey(button) {
    button.closest('.form-group').remove();
    markDirty();
}

// Custom prompts
function addCustomPrompt() {
    const name = prompt('Enter prompt name:');
    if (!name) return;
    
    const id = 'custom-' + Date.now();
    const preset = {
        name: name,
        system: 'Enter your custom system prompt here...'
    };
    
    addPromptToList(id, preset, true);
    markDirty();
}

function removePrompt(id) {
    const promptItem = document.querySelector(`[data-prompt-id="${id}"]`).closest('.prompt-item');
    promptItem.remove();
    markDirty();
}

// Update usage statistics
async function updateUsageStats() {
    try {
        const stats = await invoke('get_usage_stats');
        
        document.getElementById('usage-today').textContent = 
            `${stats.usedToday} / ${stats.limitPerDay}`;
        document.getElementById('usage-minute').textContent = 
            `${stats.usedThisMinute} / ${stats.limitPerMinute}`;
            
        const progress = (stats.usedToday / stats.limitPerDay) * 100;
        document.getElementById('usage-progress').style.width = `${progress}%`;
    } catch (error) {
        console.error('Failed to load usage stats:', error);
    }
}

// Save configuration
async function saveConfig() {
    // Validate all fields
    if (!validateApiKey() || !validateUrl()) {
        showError('Please fix validation errors before saving');
        return;
    }
    
    // Gather form data
    const formData = {
        general: {
            autoStart: document.getElementById('auto-start').checked,
            showNotifications: document.getElementById('show-notifications').checked
        },
        behavior: {
            preserveClipboard: document.getElementById('preserve-clipboard').checked,
            autoSplitLongText: document.getElementById('auto-split-text').checked
        },
        api: {
            apiKey: document.getElementById('api-key').value,
            endpoint: document.getElementById('api-endpoint').value,
            model: document.getElementById('model').value,
            temperature: parseFloat(document.getElementById('temperature').value)
        },
        hotkey: {
            translate: document.getElementById('translate-hotkey').value,
            cancel: document.getElementById('cancel-hotkey').value,
            alternatives: Array.from(document.querySelectorAll('#alternative-hotkeys input')).map(i => i.value).filter(v => v)
        },
        prompt: {
            activePreset: document.getElementById('active-prompt').value,
            custom: gatherCustomPrompts()
        },
        limits: {
            maxTextLength: parseInt(document.getElementById('max-text-length').value),
            requestsPerMinute: parseInt(document.getElementById('requests-per-minute').value),
            requestsPerDay: parseInt(document.getElementById('requests-per-day').value)
        }
    };
    
    try {
        await invoke('save_config', { config: formData });
        isDirty = false;
        showSuccess('Configuration saved successfully!');
        
        // Close after a short delay
        setTimeout(closeWindow, 1000);
    } catch (error) {
        showError('Failed to save configuration: ' + error);
    }
}

// Gather custom prompts
function gatherCustomPrompts() {
    const customPrompts = {};
    
    document.querySelectorAll('[data-prompt-id^="custom-"]').forEach(textarea => {
        const id = textarea.dataset.promptId;
        const name = textarea.closest('.prompt-item').querySelector('h4').textContent.replace(' (Custom)', '');
        customPrompts[id] = {
            name: name,
            system: textarea.value
        };
    });
    
    return customPrompts;
}

// Reset to defaults
async function resetToDefaults() {
    if (!confirm('Are you sure you want to reset all settings to defaults?')) {
        return;
    }
    
    try {
        await invoke('reset_config_to_defaults');
        await loadConfig();
        showSuccess('Configuration reset to defaults');
    } catch (error) {
        showError('Failed to reset configuration: ' + error);
    }
}

// Mark form as dirty
function markDirty() {
    isDirty = true;
}

// Close window
async function closeWindow() {
    if (isDirty) {
        const response = await confirm('You have unsaved changes. Do you want to save before closing?');
        if (response) {
            await saveConfig();
        }
    }
    
    await appWindow.close();
}

// UI notifications
function showError(message) {
    showNotification(message, 'error');
}

function showSuccess(message) {
    showNotification(message, 'success');
}

function showNotification(message, type) {
    // Create notification element
    const notification = document.createElement('div');
    notification.className = `notification ${type}`;
    notification.textContent = message;
    notification.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        padding: 12px 20px;
        border-radius: 4px;
        color: white;
        background: var(--${type});
        box-shadow: 0 2px 10px rgba(0,0,0,0.2);
        z-index: 1000;
        animation: slideIn 0.3s ease;
    `;
    
    document.body.appendChild(notification);
    
    // Remove after delay
    setTimeout(() => {
        notification.style.animation = 'slideOut 0.3s ease';
        setTimeout(() => notification.remove(), 300);
    }, 3000);
}

// Add animations
const style = document.createElement('style');
style.textContent = `
    @keyframes slideIn {
        from { transform: translateX(100%); opacity: 0; }
        to { transform: translateX(0); opacity: 1; }
    }
    @keyframes slideOut {
        from { transform: translateX(0); opacity: 1; }
        to { transform: translateX(100%); opacity: 0; }
    }
`;
document.head.appendChild(style);