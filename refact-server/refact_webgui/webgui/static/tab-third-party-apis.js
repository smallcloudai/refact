// Third-party API providers management
import { general_error } from './error.js';
let show_toast = false;

// Provider configuration with their available models
// This will be populated from litellm
let PROVIDERS = {};

// Store the configuration
let apiConfig = {
    providers: {}
};

// Initialize the third-party API widget
export async function init(general_error) {
    let req = await fetch('/tab-third-party-apis.html');
    document.querySelector('#third-party-apis').innerHTML = await req.text();

    await loadProvidersFromLiteLLM();
    loadConfiguration();
    initializeProvidersList();

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

async function loadProvidersFromLiteLLM() {
    try {
        const response = await fetch("/tab-third-party-apis-get-providers");
        if (!response.ok) {
            throw new Error("Failed to load providers from litellm");
        }

        const data = await response.json();
        const providers = data.providers || [];
        const providersModels = data.models || {};

        PROVIDERS = {};
        Object.entries(data).forEach(([providerId, providerModels]) => {
            PROVIDERS[providerId] = {
                name: providerId.split('_').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' '),
                models: providerModels
            };
        });

    } catch (error) {
        console.error("Error loading providers from litellm:", error);
        general_error(error);
    }
}

function initializeProvidersList() {
    const providersContainer = document.querySelector('#providers-container');
    providersContainer.innerHTML = '';

    Object.entries(apiConfig.providers).forEach(([providerId, providerConfig]) => {
        const providerCard = document.createElement('div');
        providerCard.className = 'card mb-3';
        providerCard.dataset.provider = providerId;
        providerCard.classList.add('api-provider-container');

        let modelsHtml = `
            <label class="form-label">Enabled Models</label>
            <div class="models-list" id="${providerId}-models-list">
                <!-- Enabled models will be populated here when configuration is loaded -->
                <div class="alert alert-info" id="${providerId}-no-enabled-models-msg">
                    No models enabled for this provider. Use the "Add Model" button below to add and enable models.
                </div>
            </div>
        `;

        providerCard.innerHTML = `
            <div class="card-header d-flex justify-content-between align-items-center">
                <h5 class="mb-0">${providerConfig.provider_name}</h5>
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

    addEventListeners();
}

function addEventListeners() {
    document.querySelectorAll('.provider-toggle').forEach(toggle => {
        toggle.addEventListener('change', function() {
            const providerId = this.dataset.provider;
            const providerBody = document.getElementById(`${providerId}-body`);
            if (this.checked) {
                providerBody.style.display = 'block';
                const modelsContainer = document.getElementById(`${providerId}-models-container`);
                if (modelsContainer) {
                    modelsContainer.style.display = 'block';
                }
            } else {
                providerBody.style.display = 'none';
                document.querySelectorAll(`#${providerId}-models-list .model-checkbox`).forEach(checkbox => {
                    checkbox.checked = false;
                });
            }
            updateConfiguration();
        });
    });

    document.querySelectorAll('.api-key-input').forEach(input => {
        input.addEventListener('blur', function() {
            const providerId = this.dataset.provider;
            updateConfiguration();

            const modelsContainer = document.getElementById(`${providerId}-models-container`);
            if (this.value) {
                modelsContainer.style.display = 'block';
            } else {
                modelsContainer.style.display = 'none';
            }
        });
    });

    document.querySelectorAll('.model-checkbox').forEach(checkbox => {
        checkbox.addEventListener('change', function() {
            updateConfiguration();
        });
    });

    const addProviderBtn = document.querySelector('.add-provider-btn');
    if (addProviderBtn) {
        addProviderBtn.addEventListener('click', function() {
            showAddProviderModal();
        });
    }

    document.querySelectorAll('.add-model-btn').forEach(button => {
        button.addEventListener('click', function() {
            const providerId = this.dataset.provider;
            showAddModelModal(providerId);
        });
    });
}

function updateConfiguration() {
    const newProviders = {};

    document.querySelectorAll('.provider-toggle:checked').forEach(toggle => {
        const providerId = toggle.dataset.provider;
        const apiKeyInput = document.getElementById(`${providerId}-api-key`);

        if (apiKeyInput && apiKeyInput.value) {
            const existingProvider = apiConfig.providers[providerId];

            newProviders[providerId] = {
                provider_name: PROVIDERS[providerId].name || providerId,
                api_key: apiKeyInput.value,
                enabled_models: existingProvider ? [...existingProvider.enabled_models] : []
            };
        }
    });

    apiConfig.providers = newProviders;
    saveConfiguration();
}

function loadConfiguration() {
    fetch("/tab-third-party-apis-get")
        .then(response => response.json())
        .then(data => {
            apiConfig = data || { providers: [] };
            updateUI();
        })
        .catch(error => {
            console.error("Error loading configuration:", error);
            general_error(error);
        });
}

// Update the UI based on loaded data
function updateUI() {
    // First, uncheck all toggles and reset provider displays
    document.querySelectorAll('.provider-toggle').forEach(toggle => {
        toggle.checked = false;
        const providerId = toggle.dataset.provider;
        document.getElementById(`${providerId}-body`).style.display = 'none';
    });

    // Update UI based on configuration
    Object.entries(apiConfig.providers).forEach(([providerId, providerConfig]) => {
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

                // Get the models list container
                const modelsList = document.getElementById(`${providerId}-models-list`);
                if (modelsList) {
                    // Clear existing models
                    modelsList.innerHTML = '';

                    // Display enabled models
                    if (enabledModels.length > 0) {
                        // Hide the "no enabled models" message if it exists
                        const noEnabledModelsMsg = document.getElementById(`${providerId}-no-enabled-models-msg`);
                        if (noEnabledModelsMsg) {
                            noEnabledModelsMsg.style.display = 'none';
                        }

                        // Add each enabled model to the list
                        enabledModels.forEach(model => {
                            const modelItem = document.createElement('div');
                            modelItem.className = 'enabled-model-item mb-2 d-flex justify-content-between align-items-center';
                            modelItem.innerHTML = `
                                <span class="model-name">${model}</span>
                                <button class="btn btn-sm btn-outline-danger remove-model-btn" 
                                        data-provider="${providerId}" 
                                        data-model="${model}">
                                    <i class="bi bi-x"></i>
                                </button>
                            `;
                            modelsList.appendChild(modelItem);

                            // Add event listener for remove button
                            const removeBtn = modelItem.querySelector('.remove-model-btn');
                            removeBtn.addEventListener('click', function() {
                                removeModel(this.dataset.provider, this.dataset.model);
                            });
                        });
                    } else {
                        // Show the "no enabled models" message
                        const noEnabledModelsMsg = document.createElement('div');
                        noEnabledModelsMsg.className = 'alert alert-info';
                        noEnabledModelsMsg.id = `${providerId}-no-enabled-models-msg`;
                        noEnabledModelsMsg.textContent = 'No models enabled for this provider. Use the "Add Model" button below to add and enable models.';
                        modelsList.appendChild(noEnabledModelsMsg);
                    }
                }
            }
        }
    });
}

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

function showAddProviderModal() {
    const providerIdSelect = document.getElementById('third-party-provider-id');
    providerIdSelect.innerHTML = '<option value="" disabled selected>Select a provider</option>';
    document.getElementById('third-party-provider-name').value = '';
    document.getElementById('third-party-provider-api-key').value = '';

    fetch("/tab-third-party-apis-get-providers")
        .then(response => response.json())
        .then(data => {
            if (data && data.providers && Array.isArray(data.providers)) {
                data.providers.forEach(provider => {
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

    providerIdSelect.addEventListener('change', function() {
        const selectedOption = this.options[this.selectedIndex];
        if (selectedOption && selectedOption.dataset.name) {
            document.getElementById('third-party-provider-name').value = selectedOption.dataset.name;
        }
    });

    const modal = new bootstrap.Modal(document.getElementById('add-third-party-provider-modal'));
    modal.show();

    document.getElementById('add-third-party-provider-submit').onclick = function() {
        addProvider();
    };
}

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

    PROVIDERS[providerId] = {
        name: providerName,
        models: []
    };

    apiConfig.providers[providerId] = {
        provider_name: providerName,
        api_key: apiKey,
        enabled_models: []
    };

    saveConfiguration();
    initializeProvidersList();
    updateUI();

    const modal = bootstrap.Modal.getInstance(document.getElementById('add-third-party-provider-modal'));
    modal.hide();

    showSuccessToast("Provider added successfully");
}

function showAddModelModal(providerId) {
    const modelIdContainer = document.getElementById('add-third-party-model-modal-id-container');
    modelIdContainer.dataset.providerId = providerId;

    if (PROVIDERS[providerId] && PROVIDERS[providerId].models && PROVIDERS[providerId].models.length > 0) {
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
        const inputHtml = `
            <label for="third-party-model-id" class="form-label">Model ID</label>
            <input type="text" class="form-control" id="third-party-model-id" placeholder="e.g., gpt-4, claude-3-opus">
            <div class="form-text">Enter the model ID as recognized by the provider.</div>
        `;

        modelIdContainer.innerHTML = inputHtml;
    }

    const modal = new bootstrap.Modal(document.getElementById('add-third-party-model-modal'));
    modal.show();

    document.getElementById('add-third-party-model-submit').onclick = function() {
        addModel();
    };
}

// Add a new model to a provider
function addModel() {
    // Get the model ID from either the input field or the select dropdown
    let modelId;
    const providerId = document.getElementById('add-third-party-model-modal-id-container').dataset.providerId;
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
    }

    // Find the provider in the configuration
    const providerConfig = apiConfig.providers[providerId];
    if (providerConfig) {
        // Check if the model is already enabled
        if (!providerConfig.enabled_models.includes(modelId)) {
            // Add the model to the enabled models
            providerConfig.enabled_models.push(modelId);

            // Update the configuration
            updateConfiguration();

            // Update the UI
            updateUI();

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
            general_error({ detail: "Model is already enabled for this provider" });
        }
    } else {
        general_error({ detail: "Provider configuration not found" });
    }
}

// Remove a model from the enabled models list
function removeModel(providerId, modelId) {
    // Find the provider in the configuration
    const providerConfig = apiConfig.providers[providerId];
    if (providerConfig) {
        // Remove the model from the enabled models
        const modelIndex = providerConfig.enabled_models.indexOf(modelId);
        if (modelIndex !== -1) {
            providerConfig.enabled_models.splice(modelIndex, 1);

            // Update the configuration
            updateConfiguration();

            // Update the UI
            updateUI();

            showSuccessToast("Model removed successfully");
        }
    }
}

export function tab_switched_here() {
    loadProvidersFromLiteLLM().then(() => {
        loadConfiguration();
        initializeProvidersList();
    }).catch(error => {
        console.error("Error reloading providers:", error);
        general_error(error);
        loadConfiguration();
    });

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