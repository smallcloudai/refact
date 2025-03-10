// Third-party API providers management
import { general_error } from './error.js';
let show_toast = false;

// Provider configuration with their available models
// This will be populated from litellm
let PROVIDERS = {};

// Store the configuration
let apiConfig = {
    providers: []
};

// Initialize the third-party API widget
export async function init(general_error) {
    let req = await fetch('/tab-third-party-apis.html');
    document.querySelector('#third-party-apis').innerHTML = await req.text();

    // Load providers and models from litellm
    await loadProvidersFromLiteLLM();

    // Initialize the providers list
    initializeProvidersList();

    // Load saved configuration
    loadConfiguration();

    // Initialize modals
    const addProviderModal = document.getElementById('add-third-party-provider-modal');
    if (addProviderModal) {
        addProviderModal._bsModal = new bootstrap.Modal(addProviderModal);

        // Add event listener for the submit button
        document.getElementById('add-third-party-provider-submit').addEventListener('click', function() {
            addProvider();
        });
    }

    const addModelModal = document.getElementById('add-third-party-model-modal');
    if (addModelModal) {
        addModelModal._bsModal = new bootstrap.Modal(addModelModal);

        // Add event listener for the submit button
        document.getElementById('add-third-party-model-submit').addEventListener('click', function() {
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

        const data = await response.json();
        const providersInfo = data.providers || [];
        const providersModels = data.models || {};

        // Convert the response to the format expected by the UI
        PROVIDERS = {};

        // First add all providers from the available providers list
        providersInfo.forEach(provider => {
            const providerId = provider.id;
            PROVIDERS[providerId] = {
                name: provider.name || providerId.split('_').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' '),
                models: providersModels[providerId] || []
            };
        });

        console.log("Loaded providers from litellm:", PROVIDERS);
    } catch (error) {
        console.error("Error loading providers from litellm:", error);
        general_error(error);
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
            }

            // Update configuration
            updateConfiguration();
        });
    });
    
    // API key inputs
    document.querySelectorAll('.api-key-input').forEach(input => {
        input.addEventListener('blur', function() {
            const providerId = this.dataset.provider;
            // Update and save configuration
            updateConfiguration();

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
            updateConfiguration();
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

// Update the configuration based on UI state
function updateConfiguration() {
    // Start with a fresh configuration
    apiConfig.providers = [];

    // Get all providers that are toggled on
    document.querySelectorAll('.provider-toggle:checked').forEach(toggle => {
        const providerId = toggle.dataset.provider;
        const apiKeyInput = document.getElementById(`${providerId}-api-key`);

        if (apiKeyInput && apiKeyInput.value) {
            // Get enabled models for this provider
            const enabledModels = [];
            document.querySelectorAll(`#${providerId}-models-list .model-checkbox:checked`).forEach(checkbox => {
                enabledModels.push(checkbox.dataset.model);
            });

            // Add provider to configuration
            apiConfig.providers.push({
                provider: providerId,
                api_key: apiKeyInput.value,
                enabled_models: enabledModels
            });
        }
    });

    // Save the configuration
    saveConfiguration();
}

// Load configuration from the server
function loadConfiguration() {
    fetch("/tab-third-party-apis-get")
        .then(response => response.json())
        .then(data => {
            // Set configuration
            apiConfig = data || { providers: [] };

            // Update UI
            updateUI();
        })
        .catch(error => {
            console.error("Error loading configuration:", error);
            general_error(error);
        });
}

// Update the UI based on loaded data
function updateUI() {
    // First, uncheck all toggles and checkboxes
    document.querySelectorAll('.provider-toggle').forEach(toggle => {
        toggle.checked = false;
        const providerId = toggle.dataset.provider;
        document.getElementById(`${providerId}-body`).style.display = 'none';
    });

    document.querySelectorAll('.model-checkbox').forEach(checkbox => {
        checkbox.checked = false;
    });

    // Update UI based on configuration
    apiConfig.providers.forEach(providerConfig => {
        const providerId = providerConfig.provider;
        const apiKey = providerConfig.api_key;
        const enabledModels = providerConfig.enabled_models || [];

        // Update API key input
        const input = document.getElementById(`${providerId}-api-key`);
        if (input) {
            input.value = apiKey;

            // Toggle provider on
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

                        // Check enabled models
                        enabledModels.forEach(model => {
                            const checkbox = document.getElementById(`${providerId}-${model}`);
                            if (checkbox) {
                                checkbox.checked = true;
                            }
                        });
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
    });
}

// Save configuration to the server
function saveConfiguration() {
    fetch("/tab-third-party-apis-save", {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(apiConfig)
    })
    .then(response => {
        if (!response.ok) {
            throw new Error("Failed to save configuration");
        }
        showSuccessToast("Configuration saved successfully");
    })
    .catch(error => {
        console.error("Error saving configuration:", error);
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
    const providerIdSelect = document.getElementById('third-party-provider-id');
    providerIdSelect.innerHTML = '<option value="" disabled selected>Select a provider</option>';
    document.getElementById('third-party-provider-name').value = '';
    document.getElementById('third-party-provider-api-key').value = '';

    // Fetch all available providers from litellm
    fetch("/tab-third-party-apis-get-providers")
        .then(response => response.json())
        .then(data => {
            // Populate the provider dropdown
            if (data && data.providers && Array.isArray(data.providers)) {
                data.providers.forEach(provider => {
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
            document.getElementById('third-party-provider-name').value = selectedOption.dataset.name;
        }
    });

    // Show the modal
    const modal = new bootstrap.Modal(document.getElementById('add-third-party-provider-modal'));
    modal.show();

    // Add event listener for the submit button
    document.getElementById('add-third-party-provider-submit').onclick = function() {
        addProvider();
    };
}

// Add a new provider
function addProvider() {
    const providerId = document.getElementById('third-party-provider-id').value.trim().toLowerCase();
    const providerName = document.getElementById('third-party-provider-name').value.trim();
    const apiKey = document.getElementById('third-party-provider-api-key').value.trim();

    if (!providerId) {
        general_error({ detail: "Provider ID is required" });
        return;
    }

    if (!providerName) {
        general_error({ detail: "Provider Name is required" });
        return;
    }

    if (!apiKey) {
        general_error({ detail: "API Key is required" });
        return;
    }

    // Add the provider to the PROVIDERS object
    PROVIDERS[providerId] = {
        name: providerName,
        models: []
    };

    // Add the provider to the configuration
    apiConfig.providers.push({
        provider: providerId,
        api_key: apiKey,
        enabled_models: []
    });

    // Save the configuration
    saveConfiguration();

    // Reinitialize the providers list
    initializeProvidersList();

    // Update the UI to show the new provider
    updateUI();

    // Close the modal
    const modal = bootstrap.Modal.getInstance(document.getElementById('add-third-party-provider-modal'));
    modal.hide();

    showSuccessToast("Provider added successfully");
}

// Show Add Model Modal
function showAddModelModal(providerId) {
    // Set the provider ID
    document.getElementById('third-party-model-provider-id').value = providerId;
    
    // Get the model ID input container
    const modelIdContainer = document.querySelector('.modal-body .mb-3');
    
    // Check if the provider has models
    if (PROVIDERS[providerId] && PROVIDERS[providerId].models && PROVIDERS[providerId].models.length > 0) {
        // Provider has models, show a combobox with option for custom input
        const selectHtml = `
            <label for="third-party-model-id" class="form-label">Model ID</label>
            <select class="form-select" id="third-party-model-id">
                <option value="" selected>-- Select a model --</option>
                ${PROVIDERS[providerId].models.map(model => `<option value="${model}">${model}</option>`).join('')}
                <option value="custom">-- Enter custom model ID --</option>
            </select>
            <input type="text" class="form-control mt-2" id="third-party-model-custom" placeholder="Enter custom model ID" style="display: none;">
            <div class="form-text">Select from available models or enter a custom model ID.</div>
        `;
        
        modelIdContainer.innerHTML = selectHtml;
        
        // Add event listener to handle custom model input
        const modelSelect = document.getElementById('third-party-model-id');
        const customInput = document.getElementById('third-party-model-custom');
        
        modelSelect.addEventListener('change', function() {
            if (this.value === 'custom') {
                customInput.style.display = 'block';
                customInput.focus();
            } else {
                customInput.style.display = 'none';
            }
        });
    } else {
        // Provider has no models, show a simple text input
        const inputHtml = `
            <label for="third-party-model-id" class="form-label">Model ID</label>
            <input type="text" class="form-control" id="third-party-model-id" placeholder="e.g., gpt-4, claude-3-opus">
            <div class="form-text">Enter the model ID as recognized by the provider.</div>
        `;
        
        modelIdContainer.innerHTML = inputHtml;
    }

    // Show the modal
    const modal = new bootstrap.Modal(document.getElementById('add-third-party-model-modal'));
    modal.show();

    // Add event listener for the submit button
    document.getElementById('add-third-party-model-submit').onclick = function() {
        addModel();
    };
}

// Add a new model to a provider
function addModel() {
    // Get the model ID from either the input field or the select dropdown
    let modelId;
    const providerId = document.getElementById('third-party-model-provider-id').value;
    const modelIdElement = document.getElementById('third-party-model-id');
    
    // Check if we're using a select element (combobox)
    if (modelIdElement.tagName === 'SELECT') {
        if (modelIdElement.value === 'custom') {
            // Get the value from the custom input field
            const customInput = document.getElementById('third-party-model-custom');
            modelId = customInput ? customInput.value.trim() : '';
        } else {
            modelId = modelIdElement.value.trim();
        }
    } else {
        // Using the regular input field
        modelId = modelIdElement.value.trim();
    }

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
            updateConfiguration();
        });

        // Close the modal
        const modalElement = document.getElementById('add-third-party-model-modal');
        const modal = bootstrap.Modal.getInstance(modalElement);
        if (modal) {
            modal.hide();
        } else if (modalElement && modalElement._bsModal) {
            modalElement._bsModal.hide();
        }
        
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
        // Load saved configuration
        loadConfiguration();
    }).catch(error => {
        console.error("Error reloading providers:", error);
        general_error(error);
        // Still load configuration even if provider loading fails
        loadConfiguration();
    });

    // Make sure the modals are properly initialized
    const addProviderModal = document.getElementById('add-third-party-provider-modal');
    if (addProviderModal && !addProviderModal._bsModal) {
        addProviderModal._bsModal = new bootstrap.Modal(addProviderModal);
    }

    const addModelModal = document.getElementById('add-third-party-model-modal');
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