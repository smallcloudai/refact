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

// Track expanded/collapsed state of providers
let expandedProviders = {};

// Initialize the third-party API widget
export async function init(general_error) {
    let req = await fetch('/tab-third-party-apis.html');
    document.querySelector('#third-party-apis').innerHTML = await req.text();

    loadProvidersFromLiteLLM();
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

function loadProvidersFromLiteLLM() {
    fetch("/tab-third-party-apis-get-providers")
        .then(response => response.json())
        .then(data => {
            PROVIDERS = {};
            Object.entries(data).forEach(([providerId, providerModels]) => {
                PROVIDERS[providerId] = {
                    name: providerId.split('_').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' '),
                    models: providerModels
                };
            });
        })
        .catch(error => {
            console.error("Error loading providers from litellm:", error);
            general_error(error);
        });
}

// Helper function to set the collapsed state of a provider
function setProviderCollapsedState(providerId, isExpanded) {
    const header = document.querySelector(`.provider-header[data-provider="${providerId}"]`);
    const body = document.getElementById(`${providerId}-body`);

    if (header && body) {
        body.style.display = isExpanded ? 'block' : 'none';

        if (isExpanded) {
            header.classList.remove('collapsed');
        } else {
            header.classList.add('collapsed');
        }
    }
}

function hasPredefinedModels(providerId) {
    return PROVIDERS[providerId] &&
           PROVIDERS[providerId].models &&
           PROVIDERS[providerId].models.length > 0;
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
            ${hasPredefinedModels(providerId) ?
              `<div class="alert alert-info mb-3">
                <i class="bi bi-info-circle"></i> This provider has predefined models. Custom models cannot be added.
               </div>` :
              `<div class="alert alert-secondary mb-3">
                <i class="bi bi-info-circle"></i> This provider doesn't have predefined models. You can add custom models.
               </div>`
            }
            <div class="models-list" id="${providerId}-models-list">
                <!-- Enabled models will be populated here when configuration is loaded -->
                <div class="alert alert-info" id="${providerId}-no-enabled-models-msg">
                    No models enabled for this provider. Use the "Add Model" button below to add and enable models.
                </div>
            </div>
        `;

        const apiKeyContainerHtml = hasPredefinedModels(providerId) ? `
            <div class="api-key-container mb-3" id="${providerId}-api-key-container">
                <label for="${providerId}-api-key" class="form-label">API Key</label>
                <input type="text" class="form-control api-key-input" id="${providerId}-api-key" data-provider="${providerId}">
            </div>
        ` : '';

        providerCard.innerHTML = `
            <div class="card-header d-flex justify-content-between align-items-center provider-header" data-provider="${providerId}">
                <h5 class="mb-0 provider-title" data-provider="${providerId}">
                    ${providerConfig.provider_name}
                </h5>
                <div class="d-flex align-items-center">
                    <div class="form-check form-switch me-2">
                        <input class="form-check-input provider-toggle" type="checkbox" id="${providerId}-toggle" data-provider="${providerId}">
                    </div>
                    <button class="btn btn-sm btn-outline-danger remove-provider-btn" data-provider="${providerId}">
                        <i class="bi bi-trash"></i>
                    </button>
                </div>
            </div>
            <div class="card-body provider-body" id="${providerId}-body" style="display: none;">
                ${apiKeyContainerHtml}
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
    // Provider toggle switch (enable/disable)
    document.querySelectorAll('.provider-toggle').forEach(toggle => {
        toggle.addEventListener('change', function() {
            const providerId = this.dataset.provider;
            updateConfiguration();
        });
    });

    // Provider header click for collapse/expand
    document.querySelectorAll('.provider-header, .provider-title').forEach(header => {
        header.addEventListener('click', function(event) {
            // Don't trigger if clicking on toggle switch or remove button
            if (event.target.classList.contains('provider-toggle') || 
                event.target.classList.contains('remove-provider-btn') ||
                event.target.closest('.remove-provider-btn') ||
                event.target.closest('.form-check')) {
                return;
            }

            const providerId = this.dataset.provider;
            const providerBody = document.getElementById(`${providerId}-body`);
            
            // Toggle visibility
            const isVisible = providerBody.style.display !== 'none';
            const newExpandedState = !isVisible;

            // Use our helper function to set the collapsed state
            setProviderCollapsedState(providerId, newExpandedState);

            // Store expanded state
            expandedProviders[providerId] = newExpandedState;
        });
    });

    // Remove provider button
    document.querySelectorAll('.remove-provider-btn').forEach(button => {
        button.addEventListener('click', function() {
            const providerId = this.dataset.provider;
            if (confirm(`Are you sure you want to remove the ${apiConfig.providers[providerId].provider_name} provider?`)) {
                delete apiConfig.providers[providerId];
                delete expandedProviders[providerId];
                saveConfiguration();
                initializeProvidersList();
                updateUI();
                showSuccessToast("Provider removed successfully");
            }
        });
    });

    // API key input
    document.querySelectorAll('.api-key-input').forEach(input => {
        input.addEventListener('blur', function() {
            const providerId = this.dataset.provider;
            updateConfiguration();
            if (hasPredefinedModels(providerId)) {
                const modelsContainer = document.getElementById(`${providerId}-models-container`);
                if (this.value) {
                    modelsContainer.style.display = 'block';
                } else {
                    modelsContainer.style.display = 'none';
                }
            }
        });
    });

    // Model checkboxes
    document.querySelectorAll('.model-checkbox').forEach(checkbox => {
        checkbox.addEventListener('change', function() {
            updateConfiguration();
        });
    });

    // Add provider button
    const addProviderBtn = document.querySelector('.add-provider-btn');
    if (addProviderBtn) {
        addProviderBtn.addEventListener('click', function() {
            showAddProviderModal();
        });
    }

    // Add model buttons
    document.querySelectorAll('.add-model-btn').forEach(button => {
        button.addEventListener('click', function() {
            const providerId = this.dataset.provider;
            showAddModelModal(providerId);
        });
    });
}

function updateConfiguration() {
    // Iterate through all providers in the current configuration
    Object.keys(apiConfig.providers).forEach(providerId => {
        const toggle = document.getElementById(`${providerId}-toggle`);
        const apiKeyInput = document.getElementById(`${providerId}-api-key`);

        if (toggle && apiKeyInput) {
            // Update the enabled state based on the toggle
            apiConfig.providers[providerId].enabled = toggle.checked;

            // Update the API key if this provider has one and it has changed
            if (apiKeyInput && apiKeyInput.value) {
                apiConfig.providers[providerId].api_key = apiKeyInput.value;
            }
        }
    });

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
        const isEnabled = providerConfig.enabled !== undefined ? providerConfig.enabled : true;

        // Update API key input if this provider has one
        const input = document.getElementById(`${providerId}-api-key`);
        if (input) {
            input.value = apiKey;
        }

        // Set toggle state based on enabled property
        const toggle = document.getElementById(`${providerId}-toggle`);
        if (toggle) {
            toggle.checked = isEnabled;

            const isExpanded = expandedProviders[providerId] !== undefined ? expandedProviders[providerId] : true;
            setProviderCollapsedState(providerId, isExpanded);

            if (!hasPredefinedModels(providerId)) {
                const modelsContainer = document.getElementById(`${providerId}-models-container`);
                if (modelsContainer) {
                    modelsContainer.style.display = 'block';
                }
            }

            const modelsList = document.getElementById(`${providerId}-models-list`);
            if (modelsList) {
                modelsList.innerHTML = '';

                if (enabledModels.length > 0) {
                    const noEnabledModelsMsg = document.getElementById(`${providerId}-no-enabled-models-msg`);
                    if (noEnabledModelsMsg) {
                        noEnabledModelsMsg.style.display = 'none';
                    }

                    enabledModels.forEach(model => {
                        const modelName = typeof model === 'string' ? model : model.model_name;
                        const supportsAgentic = typeof model === 'object' && model.supports_agentic;
                        const supportsClicks = typeof model === 'object' && model.supports_clicks;
                        const hasCustomConfig = typeof model === 'object' && model.custom_model_config;

                        let capabilitiesBadges = '';
                        if (supportsAgentic) {
                            capabilitiesBadges += '<span class="badge bg-info me-1" title="Supports Agentic Mode">Agent</span>';
                        }
                        if (supportsClicks) {
                            capabilitiesBadges += '<span class="badge bg-success me-1" title="Supports Click Interactions">Clicks</span>';
                        }
                        if (hasCustomConfig) {
                            capabilitiesBadges += '<span class="badge bg-warning me-1" title="Has Custom Configuration">Custom</span>';
                        }

                        const modelItem = document.createElement('div');
                        modelItem.className = 'enabled-model-item mb-2 d-flex justify-content-between align-items-center';
                            modelItem.innerHTML = `
                                <div class="d-flex align-items-center model-info" data-provider="${providerId}" data-model="${modelName}">
                                    <span class="model-name">${modelName}</span>
                                    <div class="ms-2">${capabilitiesBadges}</div>
                                </div>
                                <button class="btn btn-sm btn-outline-danger remove-model-btn" 
                                        data-provider="${providerId}" 
                                        data-model="${modelName}">
                                    <i class="bi bi-x"></i>
                                </button>
                            `;
                        modelsList.appendChild(modelItem);

                        // Add click event to show model details if it has custom config
                        if (hasCustomConfig) {
                            const modelInfo = modelItem.querySelector('.model-info');
                            modelInfo.style.cursor = 'pointer';
                            modelInfo.title = 'Click to view custom configuration';
                            modelInfo.addEventListener('click', function() {
                                showEditModelModal(this.dataset.provider, this.dataset.model);
                            });
                        }

                        const removeBtn = modelItem.querySelector('.remove-model-btn');
                        removeBtn.addEventListener('click', function() {
                            removeModel(this.dataset.provider, this.dataset.model);
                        });
                    });
                } else {
                    const noEnabledModelsMsg = document.createElement('div');
                    noEnabledModelsMsg.className = 'alert alert-info';
                    noEnabledModelsMsg.id = `${providerId}-no-enabled-models-msg`;
                    noEnabledModelsMsg.textContent = 'No models enabled for this provider. Use the "Add Model" button below to add and enable models.';
                    modelsList.appendChild(noEnabledModelsMsg);
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

    // Hide API key field by default until a provider is selected
    const apiKeyContainer = document.getElementById('third-party-provider-api-key-container');
    apiKeyContainer.style.display = 'block';

    Object.entries(PROVIDERS).forEach(([providerId, providerInfo]) => {
        const option = document.createElement('option');
        option.value = providerId;
        option.textContent = providerInfo.name;
        option.dataset.name = providerInfo.name;
        option.dataset.noApiKey = providerInfo.models.length > 0 ? 'false' : 'true';
        providerIdSelect.appendChild(option);
    });

    providerIdSelect.addEventListener('change', function() {
        const selectedOption = this.options[this.selectedIndex];
        if (selectedOption && selectedOption.dataset.name) {
            document.getElementById('third-party-provider-name').value = selectedOption.dataset.name;
            if (selectedOption.dataset.noApiKey === 'true') {
                apiKeyContainer.style.display = 'none';
            } else {
                apiKeyContainer.style.display = 'block';
            }
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

    // Check if the selected provider requires an API key
    const providerSelect = document.getElementById('third-party-provider-id');
    const selectedOption = providerSelect.options[providerSelect.selectedIndex];
    const requiresApiKey = selectedOption && selectedOption.dataset.noApiKey !== 'true';

    if (!providerId) {
        const error_message = "Provider ID is required"
        console.error(error_message);
        general_error(error_message);
        return;
    }

    if (!providerName) {
        const error_message = "Provider Name is required"
        console.error(error_message);
        general_error(error_message);
        return;
    }

    if (requiresApiKey && !apiKey) {
        const error_message = "API Key is required"
        console.error(error_message);
        general_error(error_message);
        return;
    }

    apiConfig.providers[providerId] = {
        provider_name: providerName,
        api_key: requiresApiKey ? apiKey : "",  // Empty string for providers that don't need API key
        enabled_models: [],
        enabled: true
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

    if (hasPredefinedModels(providerId)) {
        const selectHtml = `
            <label for="third-party-model-id" class="form-label">Model ID</label>
            <select class="form-select" id="third-party-model-id">
                <option value="" selected>-- Select a model --</option>
                ${PROVIDERS[providerId].models.map(model => `<option value="${model}">${model}</option>`).join('')}
            </select>
            <div class="form-text mb-3">Select from available models for this provider.</div>

            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-agentic">
                <label class="form-check-label" for="third-party-model-supports-agentic">
                    Supports Agentic Mode
                </label>
                <div class="form-text">Enable if this model supports autonomous agent functionality.</div>
            </div>

            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-clicks">
                <label class="form-check-label" for="third-party-model-supports-clicks">
                    Supports Clicks
                </label>
                <div class="form-text">Enable if this model supports click interactions.</div>
            </div>
        `;

        modelIdContainer.innerHTML = selectHtml;
    } else {
        const inputHtml = `
            <label for="third-party-model-id" class="form-label">Model ID</label>
            <input type="text" class="form-control" id="third-party-model-id" placeholder="e.g., gpt-4, claude-3-opus">
            <div class="form-text mb-3">Enter the model ID as recognized by the provider.</div>

            <div class="mb-3">
                <label for="custom-model-api-key" class="form-label">API Key</label>
                <input type="text" class="form-control" id="custom-model-api-key" placeholder="Enter API key for this model">
            </div>

            <div class="mb-3">
                <label for="custom-model-n-ctx" class="form-label">Context Size (n_ctx)</label>
                <input type="number" class="form-control" id="custom-model-n-ctx" placeholder="e.g., 8192" min="1024" step="1024" value="8192">
                <div class="form-text">Maximum number of tokens the model can process.</div>
            </div>

            <div class="form-check mb-3">
                <input class="form-check-input" type="checkbox" id="custom-model-supports-tools">
                <label class="form-check-label" for="custom-model-supports-tools">
                    Supports Tools
                </label>
                <div class="form-text">Enable if this model supports function calling/tools.</div>
            </div>

            <div class="form-check mb-3">
                <input class="form-check-input" type="checkbox" id="custom-model-supports-multimodality">
                <label class="form-check-label" for="custom-model-supports-multimodality">
                    Supports Multimodality
                </label>
                <div class="form-text">Enable if this model supports images and other media types.</div>
            </div>

            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-agentic">
                <label class="form-check-label" for="third-party-model-supports-agentic">
                    Supports Agentic Mode
                </label>
                <div class="form-text">Enable if this model supports autonomous agent functionality.</div>
            </div>

            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-clicks">
                <label class="form-check-label" for="third-party-model-supports-clicks">
                    Supports Clicks
                </label>
                <div class="form-text">Enable if this model supports click interactions.</div>
            </div>

            <div class="mb-3">
                <label for="custom-model-tokenizer-uri" class="form-label">Tokenizer URI (Optional)</label>
                <input type="text" class="form-control" id="custom-model-tokenizer-uri" placeholder="e.g., https://huggingface.co/model/tokenizer.json">
                <div class="form-text">URI to the tokenizer for this model. Leave empty to use default.</div>
            </div>
        `;

        modelIdContainer.innerHTML = inputHtml;
    }

    document.getElementById('add-third-party-model-modal-label').textContent = 'Add Model';
    document.getElementById('add-third-party-model-submit').textContent = 'Add Model';
    document.getElementById('add-third-party-model-submit').onclick = function() {
        addModel();
    };

    const modal = new bootstrap.Modal(document.getElementById('add-third-party-model-modal'));
    modal.show();
}

// Add a new model to a provider
function addModel() {
    // Get the model ID from either the input field or the select dropdown
    let modelId;
    const providerId = document.getElementById('add-third-party-model-modal-id-container').dataset.providerId;
    const modelIdElement = document.getElementById('third-party-model-id');

    if (!modelIdElement) {
        return;
    }

    // Get the values of the capability checkboxes
    const supportsAgentic = document.getElementById('third-party-model-supports-agentic').checked;
    const supportsClicks = document.getElementById('third-party-model-supports-clicks').checked;

    // Check if we're using a select element (combobox) or an input field
    if (modelIdElement.tagName === 'SELECT') {
        modelId = modelIdElement.value.trim();
    } else {
        // Using the regular input field for custom providers
        modelId = modelIdElement.value.trim();
    }

    if (!modelId) {
        const error_message = "Model ID is required";
        console.error(error_message);
        general_error(error_message);
        return;
    }

    // Find the provider in the configuration
    const providerConfig = apiConfig.providers[providerId];
    if (providerConfig) {
        // Check if the model is already enabled (by model name)
        const modelExists = providerConfig.enabled_models.some(model =>
            typeof model === 'string' ? model === modelId : model.model_name === modelId
        );

        if (!modelExists) {
            const modelConfig = {
                model_name: modelId,
                supports_agentic: supportsAgentic,
                supports_clicks: supportsClicks
            };

            if (!hasPredefinedModels(providerId)) {
                const customApiKey = document.getElementById('custom-model-api-key').value.trim();
                const customNCtx = parseInt(document.getElementById('custom-model-n-ctx').value.trim(), 10);
                const customSupportsTools = document.getElementById('custom-model-supports-tools').checked;
                const customSupportsMultimodality = document.getElementById('custom-model-supports-multimodality').checked;
                const customTokenizerUri = document.getElementById('custom-model-tokenizer-uri').value.trim();

                // Validate required fields
                if (!customApiKey) {
                    const error_message = "API Key is required for custom model configuration";
                    console.error(error_message);
                    general_error(error_message);
                    return;
                }

                if (isNaN(customNCtx) || customNCtx < 1024) {
                    const error_message = "Context size must be a valid number greater than or equal to 1024";
                    console.error(error_message);
                    general_error(error_message);
                    return;
                }

                // Create the custom model configuration
                modelConfig.custom_model_config = {
                    api_key: customApiKey,
                    n_ctx: customNCtx,
                    supports_tools: customSupportsTools,
                    supports_multimodality: customSupportsMultimodality,
                    tokenizer_uri: customTokenizerUri || null
                };
            }

            // Add the model config to the enabled models
            providerConfig.enabled_models.push(modelConfig);

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
            const error_message = "Model is already enabled for this provider";
            console.error(error_message);
            general_error(error_message);
        }
    } else {
        const error_message = "Provider configuration not found"
        console.error(error_message);
        general_error(error_message);
    }
}

function showEditModelModal(providerId, modelId) {
    const providerConfig = apiConfig.providers[providerId];
    if (!providerConfig) {
        general_error("Provider configuration not found");
        return;
    }

    const modelObj = providerConfig.enabled_models.find(model =>
        typeof model === 'string' ? model === modelId : model.model_name === modelId
    );

    if (!modelObj || typeof modelObj === 'string' || !modelObj.custom_model_config) {
        general_error("Custom model configuration not found");
        return;
    }

    const customConfig = modelObj.custom_model_config;

    const modelIdContainer = document.getElementById('add-third-party-model-modal-id-container');
    modelIdContainer.dataset.providerId = providerId;
    modelIdContainer.dataset.modelId = modelId;
    modelIdContainer.dataset.isEdit = 'true';

    if (hasPredefinedModels(providerId)) {
        const editHtml = `
            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-agentic" ${modelObj.supports_agentic ? 'checked' : ''}>
                <label class="form-check-label" for="third-party-model-supports-agentic">
                    Supports Agentic Mode
                </label>
                <div class="form-text">Enable if this model supports autonomous agent functionality.</div>
            </div>

            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-clicks" ${modelObj.supports_clicks ? 'checked' : ''}>
                <label class="form-check-label" for="third-party-model-supports-clicks">
                    Supports Clicks
                </label>
                <div class="form-text">Enable if this model supports click interactions.</div>
            </div>
        `;

        modelIdContainer.innerHTML = editHtml;
        document.getElementById('add-third-party-model-modal-label').textContent = 'Edit Model';
    } else {
        const editHtml = `
            <div class="mb-3">
                <label for="custom-model-api-key" class="form-label">API Key</label>
                <input type="text" class="form-control" id="custom-model-api-key" value="${customConfig.api_key}" placeholder="Enter API key for this model">
            </div>

            <div class="mb-3">
                <label for="custom-model-n-ctx" class="form-label">Context Size (n_ctx)</label>
                <input type="number" class="form-control" id="custom-model-n-ctx" value="${customConfig.n_ctx}" placeholder="e.g., 8192" min="1024" step="1024">
                <div class="form-text">Maximum number of tokens the model can process.</div>
            </div>

            <div class="form-check mb-3">
                <input class="form-check-input" type="checkbox" id="custom-model-supports-tools" ${customConfig.supports_tools ? 'checked' : ''}>
                <label class="form-check-label" for="custom-model-supports-tools">
                    Supports Tools
                </label>
                <div class="form-text">Enable if this model supports function calling/tools.</div>
            </div>

            <div class="form-check mb-3">
                <input class="form-check-input" type="checkbox" id="custom-model-supports-multimodality" ${customConfig.supports_multimodality ? 'checked' : ''}>
                <label class="form-check-label" for="custom-model-supports-multimodality">
                    Supports Multimodality
                </label>
                <div class="form-text">Enable if this model supports images and other media types.</div>
            </div>

            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-agentic" ${modelObj.supports_agentic ? 'checked' : ''}>
                <label class="form-check-label" for="third-party-model-supports-agentic">
                    Supports Agentic Mode
                </label>
                <div class="form-text">Enable if this model supports autonomous agent functionality.</div>
            </div>

            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-clicks" ${modelObj.supports_clicks ? 'checked' : ''}>
                <label class="form-check-label" for="third-party-model-supports-clicks">
                    Supports Clicks
                </label>
                <div class="form-text">Enable if this model supports click interactions.</div>
            </div>

            <div class="mb-3">
                <label for="custom-model-tokenizer-uri" class="form-label">Tokenizer URI (Optional)</label>
                <input type="text" class="form-control" id="custom-model-tokenizer-uri" value="${customConfig.tokenizer_uri || ''}" placeholder="e.g., https://huggingface.co/model/tokenizer.json">
                <div class="form-text">URI to the tokenizer for this model. Leave empty to use default.</div>
            </div>
        `;

        modelIdContainer.innerHTML = editHtml;
        document.getElementById('add-third-party-model-modal-label').textContent = 'Edit Custom Model';
    }

    document.getElementById('add-third-party-model-submit').textContent = 'Save Changes';
    document.getElementById('add-third-party-model-submit').onclick = function() {
        updateModel();
    };

    const modal = new bootstrap.Modal(document.getElementById('add-third-party-model-modal'));
    modal.show();
}

function updateModel() {
    const modelIdContainer = document.getElementById('add-third-party-model-modal-id-container');
    const providerId = modelIdContainer.dataset.providerId;
    const modelId = modelIdContainer.dataset.modelId;

    const providerConfig = apiConfig.providers[providerId];
    if (!providerConfig) {
        const error_message = "No provider in config, can't update model";
        console.error(error_message);
        general_error(error_message);
    }

    const modelIndex = providerConfig.enabled_models.findIndex(model =>
        typeof model === 'string' ? model === modelId : model.model_name === modelId
    );

    if (modelIndex === -1) {
        const error_message = "No model in provider in config, can't update model";
        console.error(error_message);
        general_error(error_message);
    }

    const modelConfig = providerConfig.enabled_models[modelIndex];

    if (typeof modelConfig === 'string') {
        const error_message = "Invalid format of model config, can't update model";
        console.error(error_message);
        general_error(error_message);
    }

    // Get the values of the capability checkboxes
    const supportsAgentic = document.getElementById('third-party-model-supports-agentic').checked;
    const supportsClicks = document.getElementById('third-party-model-supports-clicks').checked;

    if (hasPredefinedModels(providerId)) {
        modelConfig.supports_agentic = supportsAgentic;
        modelConfig.supports_clicks = supportsClicks;
        modelConfig.custom_model_config = null;
    } else {
        // Get the custom model configuration values
        const customApiKey = document.getElementById('custom-model-api-key').value.trim();
        const customNCtx = parseInt(document.getElementById('custom-model-n-ctx').value.trim(), 10);
        const customSupportsTools = document.getElementById('custom-model-supports-tools').checked;
        const customSupportsMultimodality = document.getElementById('custom-model-supports-multimodality').checked;
        const customTokenizerUri = document.getElementById('custom-model-tokenizer-uri').value.trim();

        // Validate required fields
        if (!customApiKey) {
            const error_message = "API Key is required for custom model configuration";
            console.error(error_message);
            general_error(error_message);
            return;
        }

        if (isNaN(customNCtx) || customNCtx < 1024) {
            const error_message = "Context size must be a valid number greater than or equal to 1024";
            console.error(error_message);
            general_error(error_message);
            return;
        }

        modelConfig.supports_agentic = supportsAgentic;
        modelConfig.supports_clicks = supportsClicks;
        modelConfig.custom_model_config = {
            api_key: customApiKey,
            n_ctx: customNCtx,
            supports_tools: customSupportsTools,
            supports_multimodality: customSupportsMultimodality,
            tokenizer_uri: customTokenizerUri || null
        };
    }

    updateConfiguration();
    updateUI();

    const modalElement = document.getElementById('add-third-party-model-modal');
    const modal = bootstrap.Modal.getInstance(modalElement);
    if (modal) {
        modal.hide();
    } else if (modalElement && modalElement._bsModal) {
        modalElement._bsModal.hide();
    }

    showSuccessToast("Model updated successfully");
}

function removeModel(providerId, modelId) {
    // Find the provider in the configuration
    const providerConfig = apiConfig.providers[providerId];
    if (providerConfig) {
        // Find the model index, handling both string models and ModelConfig objects
        const modelIndex = providerConfig.enabled_models.findIndex(model =>
            typeof model === 'string' ? model === modelId : model.model_name === modelId
        );

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
    try {
        loadConfiguration();
        initializeProvidersList();
    } catch (error) {
        console.error("Error reloading providers:", error);
        general_error(error);
        loadConfiguration();
    };

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