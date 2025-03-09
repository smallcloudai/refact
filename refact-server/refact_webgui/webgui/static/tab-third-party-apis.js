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

    // Initialize modals
    const addProviderModal = document.getElementById('add-provider-modal');
    if (addProviderModal) {
        addProviderModal._bsModal = new bootstrap.Modal(addProviderModal);

        // Add event listener for the submit button
        document.getElementById('add-provider-submit').addEventListener('click', function() {
            addProvider();
        });
    }

    const addModelModal = document.getElementById('add-model-modal');
    if (addModelModal) {
        addModelModal._bsModal = new bootstrap.Modal(addModelModal);

        // Add event listener for the submit button
        document.getElementById('add-model-submit').addEventListener('click', function() {
            addModel();
        });
    }
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
            // Skip providers with no models
            if (!models || models.length === 0) {
                continue;
            }

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
        providerCard.dataset.provider = providerId;

        // Add provider-specific styling class
        providerCard.classList.add('api-provider-container');

        let modelsHtml = '';
        if (provider.models && provider.models.length > 0) {
            modelsHtml = `
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
            `;
        } else {
            modelsHtml = `
                <div class="alert alert-info" id="${providerId}-no-models-msg">
                    No models available for this provider. Use the "Add Model" button to add models.
                </div>
            `;
        }

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
                    ${modelsHtml}
                    <div class="mt-3">
                        <button class="btn btn-sm btn-outline-primary add-model-btn" data-provider="${providerId}">
                            <i class="bi bi-plus-circle"></i> Add Model
                        </button>
                    </div>
                </div>
            </div>
        `;
        providersContainer.appendChild(providerCard);
    });

    // Add "Add Provider" button
    const addProviderCard = document.createElement('div');
    addProviderCard.className = 'card mb-3 add-provider-card';
    addProviderCard.innerHTML = `
        <div class="card-body text-center py-3">
            <button class="btn btn-primary add-provider-btn">
                <i class="bi bi-plus-circle"></i> Add Provider
            </button>
        </div>
    `;
    providersContainer.appendChild(addProviderCard);

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

                // Always make sure the models container is visible when provider is toggled on
                const modelsContainer = document.getElementById(`${providerId}-models-container`);
                if (modelsContainer) {
                    modelsContainer.style.display = 'block';
                }
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

            // Always show the models container when an API key is provided
            const modelsContainer = document.getElementById(`${providerId}-models-container`);
            if (this.value) {
                modelsContainer.style.display = 'block';
            } else {
                modelsContainer.style.display = 'none';
            }
        });
    });

    // Model checkboxes
    document.querySelectorAll('.model-checkbox').forEach(checkbox => {
        checkbox.addEventListener('change', function() {
            updateEnabledModels();
        });
    });

    // Add Provider button
    const addProviderBtn = document.querySelector('.add-provider-btn');
    if (addProviderBtn) {
        addProviderBtn.addEventListener('click', function() {
            showAddProviderModal();
        });
    }

    // Add Model buttons
    document.querySelectorAll('.add-model-btn').forEach(button => {
        button.addEventListener('click', function() {
            const providerId = this.dataset.provider;
            showAddModelModal(providerId);
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

                    // Only show models container if the provider has models
                    const modelsContainer = document.getElementById(`${providerId}-models-container`);
                    if (modelsContainer) {
                        // Check if provider exists in PROVIDERS and has models
                        if (PROVIDERS[providerId] && PROVIDERS[providerId].models && PROVIDERS[providerId].models.length > 0) {
                            modelsContainer.style.display = 'block';
                        } else {
                            modelsContainer.style.display = 'none';

                            // Add a message if there are no models
                            const noModelsMsg = document.createElement('div');
                            noModelsMsg.className = 'alert alert-info mt-3';
                            noModelsMsg.textContent = 'No models available for this provider. Use the "Add Model" button to add models.';

                            // Check if message already exists
                            if (!document.getElementById(`${providerId}-no-models-msg`)) {
                                noModelsMsg.id = `${providerId}-no-models-msg`;
                                document.getElementById(`${providerId}-body`).appendChild(noModelsMsg);
                            }
                        }
                    }
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

// Show Add Provider Modal
function showAddProviderModal() {
    // Clear previous values
    const providerIdSelect = document.getElementById('provider-id');
    providerIdSelect.innerHTML = '<option value="" disabled selected>Select a provider</option>';
    document.getElementById('provider-name').value = '';
    document.getElementById('provider-api-key').value = '';

    // Fetch all available providers from litellm
    fetch("/tab-third-party-apis-get-all-providers")
        .then(response => response.json())
        .then(data => {
            // Populate the provider dropdown
            if (data && Array.isArray(data)) {
                data.forEach(provider => {
                    // Skip providers that are already added
                    if (PROVIDERS[provider.id]) {
                        return;
                    }

                    const option = document.createElement('option');
                    option.value = provider.id;
                    option.textContent = provider.name || provider.id;
                    option.dataset.name = provider.name || '';
                    providerIdSelect.appendChild(option);
                });
            }
        })
        .catch(error => {
            console.error("Error fetching available providers:", error);
            general_error(error);
        });

    // Add event listener for provider selection to auto-fill the name
    providerIdSelect.addEventListener('change', function() {
        const selectedOption = this.options[this.selectedIndex];
        if (selectedOption && selectedOption.dataset.name) {
            document.getElementById('provider-name').value = selectedOption.dataset.name;
        }
    });

    // Show the modal
    const modal = new bootstrap.Modal(document.getElementById('add-provider-modal'));
    modal.show();

    // Add event listener for the submit button
    document.getElementById('add-provider-submit').onclick = function() {
        addProvider();
    };
}

// Add a new provider
function addProvider() {
    const providerId = document.getElementById('provider-id').value.trim().toLowerCase();
    const providerName = document.getElementById('provider-name').value.trim();
    const apiKey = document.getElementById('provider-api-key').value.trim();

    if (!providerId) {
        general_error({ detail: "Provider ID is required" });
        return;
    }

    if (!providerName) {
        general_error({ detail: "Provider Name is required" });
        return;
    }

    // Add the provider to the PROVIDERS object
    PROVIDERS[providerId] = {
        name: providerName,
        models: []
    };

    // Save the provider to the server
    fetch("/tab-third-party-apis-add-provider", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            providerId: providerId,
            providerName: providerName,
            apiKey: apiKey
        })
    })
    .then(response => {
        if (!response.ok) {
            throw new Error("Failed to save provider");
        }
        return response.json();
    })
    .then(data => {
        // Save the API key if provided
        if (apiKey) {
            apiKeys[providerId] = apiKey;
            saveApiKeys();
        }

        // Reinitialize the providers list
        initializeProvidersList();

        // Update the UI to show the new provider
        updateUI();

        // If API key was provided, toggle the provider on
        if (apiKey) {
            const toggle = document.getElementById(`${providerId}-toggle`);
            if (toggle) {
                toggle.checked = true;
                const event = new Event('change');
                toggle.dispatchEvent(event);

                // Make sure the models container is visible
                const modelsContainer = document.getElementById(`${providerId}-models-container`);
                if (modelsContainer) {
                    modelsContainer.style.display = 'block';
                }
            }
        }

        // Close the modal
        const modal = bootstrap.Modal.getInstance(document.getElementById('add-provider-modal'));
        modal.hide();

        showSuccessToast("Provider added successfully");
    })
    .catch(error => {
        console.error("Error saving provider:", error);
        general_error(error);
    });
}

// Show Add Model Modal
function showAddModelModal(providerId) {
    // Clear previous values
    document.getElementById('model-id').value = '';
    document.getElementById('model-provider-id').value = providerId;

    // Show the modal
    const modal = new bootstrap.Modal(document.getElementById('add-model-modal'));
    modal.show();

    // Add event listener for the submit button
    document.getElementById('add-model-submit').onclick = function() {
        addModel();
    };
}

// Add a new model to a provider
function addModel() {
    const modelId = document.getElementById('model-id').value.trim();
    const providerId = document.getElementById('model-provider-id').value;

    if (!modelId) {
        general_error({ detail: "Model ID is required" });
        return;
    }

    // Initialize models array if it doesn't exist
    if (!PROVIDERS[providerId].models) {
        PROVIDERS[providerId].models = [];
    }

    // Add the model to the provider's models array if it doesn't already exist
    if (!PROVIDERS[providerId].models.includes(modelId)) {
        PROVIDERS[providerId].models.push(modelId);

        // Remove the "no models" message if it exists
        const noModelsMsg = document.getElementById(`${providerId}-no-models-msg`);
        if (noModelsMsg) {
            noModelsMsg.remove();
        }

        // Create the models list container if it doesn't exist
        let modelsList = document.getElementById(`${providerId}-models-list`);
        if (!modelsList) {
            const modelsContainer = document.getElementById(`${providerId}-models-container`);

            // Add the label
            const label = document.createElement('label');
            label.className = 'form-label';
            label.textContent = 'Available Chat Models';
            modelsContainer.insertBefore(label, modelsContainer.firstChild);

            // Create the models list
            modelsList = document.createElement('div');
            modelsList.id = `${providerId}-models-list`;
            modelsList.className = 'models-list';
            modelsContainer.insertBefore(modelsList, modelsContainer.querySelector('.mt-3'));
        }

        // Add the new model to the list
        const modelCheckbox = document.createElement('div');
        modelCheckbox.className = 'form-check mb-2';
        modelCheckbox.innerHTML = `
            <input class="form-check-input model-checkbox" type="checkbox" id="${providerId}-${modelId}" data-provider="${providerId}" data-model="${modelId}">
            <label class="form-check-label" for="${providerId}-${modelId}">
                ${modelId}
            </label>
        `;
        modelsList.appendChild(modelCheckbox);

        // Add event listener to the new checkbox
        const checkbox = modelCheckbox.querySelector('.model-checkbox');
        checkbox.addEventListener('change', function() {
            updateEnabledModels();
        });

        // Close the modal
        const modal = bootstrap.Modal.getInstance(document.getElementById('add-model-modal'));
        modal.hide();
        
        showSuccessToast("Model added successfully");
    } else {
        general_error({ detail: "Model already exists for this provider" });
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

    // Make sure the modals are properly initialized
    const addProviderModal = document.getElementById('add-provider-modal');
    if (addProviderModal && !addProviderModal._bsModal) {
        addProviderModal._bsModal = new bootstrap.Modal(addProviderModal);
    }

    const addModelModal = document.getElementById('add-model-modal');
    if (addModelModal && !addModelModal._bsModal) {
        addModelModal._bsModal = new bootstrap.Modal(addModelModal);
    }
}

export function tab_switched_away() {
    // Nothing to do when switching away
}

export function tab_update_each_couple_of_seconds() {
    // Nothing to update periodically
}