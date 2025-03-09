// Third-party API providers management
import { general_error } from './error.js';
let show_toast = false;

// Provider configuration with their available models
// This will be populated from litellm
let PROVIDERS = {};

// Store the enabled models
let enabledModels = {};
// Store API keys
let apiKeys = {};

// Initialize the third-party API widget
export async function init(general_error) {
    let req = await fetch('/tab-third-party-apis.html');
    document.querySelector('#third-party-apis').innerHTML = await req.text();
    
    // Load providers and models from litellm
    await loadProvidersFromLiteLLM();
    
    // Initialize the providers list
    initializeProvidersList();
    
    // Load saved API keys and enabled models
    loadApiKeysAndEnabledModels();
}

// Load providers and models from litellm
async function loadProvidersFromLiteLLM() {
    try {
        const response = await fetch("/tab-third-party-apis-get-providers");
        if (!response.ok) {
            throw new Error("Failed to load providers from litellm");
        }
        
        const providersModels = await response.json();
        
        // Convert the response to the format expected by the UI
        PROVIDERS = {};
        for (const [providerId, models] of Object.entries(providersModels)) {
            // Format provider name for display (capitalize first letter of each word)
            const formattedName = providerId
                .split('_')
                .map(word => word.charAt(0).toUpperCase() + word.slice(1))
                .join(' ');
                
            PROVIDERS[providerId] = {
                name: formattedName,
                models: models
            };
        }
        
        console.log("Loaded providers from litellm:", PROVIDERS);
    } catch (error) {
        console.error("Error loading providers from litellm:", error);
        general_error(error);
        
        // Fallback to default providers if litellm is not available
        PROVIDERS = {
            openai: {
                name: "OpenAI",
                models: ["gpt-3.5-turbo", "gpt-4", "gpt-4-turbo", "gpt-4o"]
            },
            anthropic: {
                name: "Anthropic",
                models: ["claude-instant-1", "claude-2", "claude-3-opus", "claude-3-sonnet", "claude-3-haiku"]
            },
            groq: {
                name: "Groq",
                models: ["llama2-70b", "mixtral-8x7b", "gemma-7b"]
            }
        };
    }
}

// Initialize the providers list
function initializeProvidersList() {
    const providersContainer = document.querySelector('#providers-container');
    providersContainer.innerHTML = '';
    
    // Create a card for each provider
    Object.keys(PROVIDERS).forEach(providerId => {
        const provider = PROVIDERS[providerId];
        const providerCard = document.createElement('div');
        providerCard.className = 'card mb-3';
        providerCard.innerHTML = `
            <div class="card-header d-flex justify-content-between align-items-center">
                <h5 class="mb-0">${provider.name}</h5>
                <div class="form-check form-switch">
                    <input class="form-check-input provider-toggle" type="checkbox" id="${providerId}-toggle" data-provider="${providerId}">
                </div>
            </div>
            <div class="card-body provider-body" id="${providerId}-body" style="display: none;">
                <div class="api-key-container mb-3" id="${providerId}-api-key-container">
                    <label for="${providerId}-api-key" class="form-label">API Key</label>
                    <input type="text" class="form-control api-key-input" id="${providerId}-api-key" data-provider="${providerId}">
                </div>
                <div class="models-container" id="${providerId}-models-container">
                    <label class="form-label">Available Chat Models</label>
                    <div class="models-list" id="${providerId}-models-list">
                        ${provider.models.map(model => `
                            <div class="form-check mb-2">
                                <input class="form-check-input model-checkbox" type="checkbox" id="${providerId}-${model}" data-provider="${providerId}" data-model="${model}">
                                <label class="form-check-label" for="${providerId}-${model}">
                                    ${model}
                                </label>
                            </div>
                        `).join('')}
                    </div>
                </div>
            </div>
        `;
        providersContainer.appendChild(providerCard);
    });
    
    // Add event listeners
    addEventListeners();
}

// Add event listeners to the UI elements
function addEventListeners() {
    // Provider toggle switches
    document.querySelectorAll('.provider-toggle').forEach(toggle => {
        toggle.addEventListener('change', function() {
            const providerId = this.dataset.provider;
            const providerBody = document.getElementById(`${providerId}-body`);
            
            if (this.checked) {
                providerBody.style.display = 'block';
            } else {
                providerBody.style.display = 'none';
                // Uncheck all models for this provider
                document.querySelectorAll(`#${providerId}-models-list .model-checkbox`).forEach(checkbox => {
                    checkbox.checked = false;
                });
                // Update enabled models
                updateEnabledModels();
            }
        });
    });
    
    // API key inputs
    document.querySelectorAll('.api-key-input').forEach(input => {
        input.addEventListener('blur', function() {
            const providerId = this.dataset.provider;
            apiKeys[providerId] = this.value;
            saveApiKeys();
            
            // If API key is provided, enable the models section
            if (this.value) {
                document.getElementById(`${providerId}-models-container`).style.display = 'block';
            } else {
                document.getElementById(`${providerId}-models-container`).style.display = 'none';
            }
        });
    });
    
    // Model checkboxes
    document.querySelectorAll('.model-checkbox').forEach(checkbox => {
        checkbox.addEventListener('change', function() {
            updateEnabledModels();
        });
    });
}

// Get model information from litellm
async function getModelInfo(providerId, modelName) {
    try {
        const response = await fetch(`/tab-third-party-apis-get-model-info?model_name=${encodeURIComponent(modelName)}&provider_name=${encodeURIComponent(providerId)}`);
        if (!response.ok) {
            throw new Error(`Failed to get model info for ${modelName}`);
        }
        return await response.json();
    } catch (error) {
        console.error(`Error getting model info for ${modelName}:`, error);
        return null;
    }
}

// Update the enabled models based on checkbox state
function updateEnabledModels() {
    enabledModels = {};
    
    document.querySelectorAll('.model-checkbox:checked').forEach(checkbox => {
        const providerId = checkbox.dataset.provider;
        const model = checkbox.dataset.model;
        
        if (!enabledModels[providerId]) {
            enabledModels[providerId] = [];
        }
        
        enabledModels[providerId].push(model);
    });
    
    saveEnabledModels();
}

// Load API keys and enabled models from the server
function loadApiKeysAndEnabledModels() {
    fetch("/tab-third-party-apis-get")
        .then(response => response.json())
        .then(data => {
            // Set API keys
            apiKeys = data.apiKeys || {};
            
            // Set enabled models
            enabledModels = data.enabledModels || {};
            
            // Update UI
            updateUI();
        })
        .catch(error => {
            console.error("Error loading API keys and enabled models:", error);
            general_error(error);
        });
}

// Update the UI based on loaded data
function updateUI() {
    // Update API key inputs
    Object.keys(apiKeys).forEach(providerId => {
        const input = document.getElementById(`${providerId}-api-key`);
        if (input) {
            input.value = apiKeys[providerId];
            
            // If API key exists, show the provider toggle and body
            if (apiKeys[providerId]) {
                const toggle = document.getElementById(`${providerId}-toggle`);
                if (toggle) {
                    toggle.checked = true;
                    document.getElementById(`${providerId}-body`).style.display = 'block';
                    document.getElementById(`${providerId}-models-container`).style.display = 'block';
                }
            }
        }
    });
    
    // Update model checkboxes
    Object.keys(enabledModels).forEach(providerId => {
        enabledModels[providerId].forEach(model => {
            const checkbox = document.getElementById(`${providerId}-${model}`);
            if (checkbox) {
                checkbox.checked = true;
            }
        });
    });
}

// Save API keys to the server
function saveApiKeys() {
    fetch("/tab-third-party-apis-save-keys", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(apiKeys)
    })
    .then(response => {
        if (!response.ok) {
            throw new Error("Failed to save API keys");
        }
        showSuccessToast("API keys saved successfully");
    })
    .catch(error => {
        console.error("Error saving API keys:", error);
        general_error(error);
    });
}

// Save enabled models to the server
function saveEnabledModels() {
    fetch("/tab-third-party-apis-save-models", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(enabledModels)
    })
    .then(response => {
        if (!response.ok) {
            throw new Error("Failed to save enabled models");
        }
        showSuccessToast("Models configuration saved successfully");
    })
    .catch(error => {
        console.error("Error saving enabled models:", error);
        general_error(error);
    });
}

// Show success toast
function showSuccessToast(message) {
    let toastDiv = document.querySelector('.third-party-apis-toast');
    const toast = bootstrap.Toast.getOrCreateInstance(toastDiv);
    
    if (!show_toast) {
        show_toast = true;
        document.querySelector('.third-party-apis-toast .toast-body').innerHTML = message;
        toast.show();
        setTimeout(function () {
            toast.hide();
            show_toast = false;
        }, 2000);
    }
}

export function tab_switched_here() {
    // Reload providers from litellm and refresh the UI
    loadProvidersFromLiteLLM().then(() => {
        // Reinitialize the providers list with the updated data
        initializeProvidersList();
        // Load saved API keys and enabled models
        loadApiKeysAndEnabledModels();
    }).catch(error => {
        console.error("Error reloading providers:", error);
        general_error(error);
        // Still load API keys and enabled models even if provider loading fails
        loadApiKeysAndEnabledModels();
    });
}

export function tab_switched_away() {
    // Nothing to do when switching away
}

export function tab_update_each_couple_of_seconds() {
    // Nothing to update periodically
}