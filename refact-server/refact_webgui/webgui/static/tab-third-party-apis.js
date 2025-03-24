// Third-party API providers management
import { general_error } from './error.js';
let show_toast = false;

// Provider default configurations with their available models
// This will be populated from litellm
let PROVIDER_DEFAULT_CONFIGS = {};

// Store the configuration
let apiConfig = {
    providers: {},
    models: {}
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
            PROVIDER_DEFAULT_CONFIGS = data;
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
    return PROVIDER_DEFAULT_CONFIGS[providerId] &&
           PROVIDER_DEFAULT_CONFIGS[providerId].length > 0;
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
            if (confirm(`Are you sure you want to remove the ${providerId} provider?`)) {
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
            apiConfig = data;
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
        // Get the first API key (for simplicity)
        const apiKey = providerConfig.api_keys && providerConfig.api_keys.length > 0 ? providerConfig.api_keys[0] : "";
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

                // Get models for this provider
                const providerModels = Object.entries(apiConfig.models)
                    .filter(([_, model]) => model.provider_id === providerId)
                    .map(([modelId, _]) => modelId);

                if (providerModels.length > 0) {
                    const noEnabledModelsMsg = document.getElementById(`${providerId}-no-enabled-models-msg`);
                    if (noEnabledModelsMsg) {
                        noEnabledModelsMsg.style.display = 'none';
                    }

                    providerModels.forEach(modelId => {
                        const modelConfig = apiConfig.models[modelId];
                        const hasCustomConfig = modelConfig.api_base !== undefined;

                        // Get default capabilities from PROVIDER_DEFAULT_CONFIGS if available
                        const providerDefaultModels = PROVIDER_DEFAULT_CONFIGS[providerId] || [];
                        const defaultConfig = providerDefaultModels.find(m => m.model_id === modelId);
                        const defaultCapabilities = defaultConfig ? defaultConfig.capabilities : null;

                        // Combine all capabilities sources
                        const capabilities = {
                            tools: modelConfig.capabilities.tools ||
                                  (defaultCapabilities && defaultCapabilities.tools) ||
                                  false,
                            multimodal: modelConfig.capabilities.multimodal ||
                                       (defaultCapabilities && defaultCapabilities.multimodal) ||
                                       false,
                            agent: modelConfig.capabilities.agent || false,
                            clicks: modelConfig.capabilities.clicks || false,
                            completion: modelConfig.capabilities.completion ||
                                       (defaultCapabilities && defaultCapabilities.completion) ||
                                       false
                        };

                        let capabilitiesBadges = '';
                        if (capabilities.agent) {
                            capabilitiesBadges += '<span class="badge bg-info me-1" title="Supports Agentic Mode">Agent</span>';
                        }
                        if (capabilities.clicks) {
                            capabilitiesBadges += '<span class="badge bg-success me-1" title="Supports Click Interactions">Clicks</span>';
                        }
                        if (capabilities.tools) {
                            capabilitiesBadges += '<span class="badge bg-secondary me-1" title="Supports Function Calling/Tools">Tools</span>';
                        }
                        if (capabilities.multimodal) {
                            capabilitiesBadges += '<span class="badge bg-primary me-1" title="Supports Images and Other Media">Multimodal</span>';
                        }
                        if (hasCustomConfig) {
                            capabilitiesBadges += '<span class="badge bg-warning me-1" title="Has Custom Configuration">Custom</span>';
                        }

                        const modelItem = document.createElement('div');
                        modelItem.className = 'enabled-model-item mb-2 d-flex justify-content-between align-items-center';
                            modelItem.innerHTML = `
                                <div class="d-flex align-items-center model-info" data-provider="${providerId}" data-model="${modelId}">
                                    <span class="model-name">${modelId}</span>
                                    <div class="ms-2">${capabilitiesBadges}</div>
                                </div>
                                <button class="btn btn-sm btn-outline-danger remove-model-btn" 
                                        data-provider="${providerId}" 
                                        data-model="${modelId}">
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

    Object.keys(PROVIDER_DEFAULT_CONFIGS).forEach((providerId) => {
        const option = document.createElement('option');
        option.value = providerId;
        option.textContent = providerId;
        option.dataset.name = providerId;
        // TODO: no need in this logic anymore
        // option.dataset.noApiKey = providerInfo.models.length > 0 ? 'false' : 'true';
        option.dataset.noApiKey = 'false';
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
        api_keys: requiresApiKey ? [apiKey] : [],  // Array of API keys
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
        const providerModels = PROVIDER_DEFAULT_CONFIGS[providerId];
        const selectHtml = `
            <label for="third-party-model-id" class="form-label">Model ID</label>
            <select class="form-select" id="third-party-model-id">
                <option value="" selected>-- Select a model --</option>
                ${providerModels.map(model => `<option value="${model.model_id}">${model.model_id}</option>`).join('')}
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

        // Add event listener to pre-fill capabilities when a model is selected
        const modelSelect = document.getElementById('third-party-model-id');
        if (modelSelect) {
            modelSelect.addEventListener('change', function() {
                const selectedModelId = this.value;
                const selectedModel = providerModels.find(model => model.model_id === selectedModelId);

                if (selectedModel) {
                    // Pre-fill agent and clicks capabilities from the model config
                    document.getElementById('third-party-model-supports-agentic').checked = 
                        selectedModel.capabilities.agent;
                    document.getElementById('third-party-model-supports-clicks').checked = 
                        selectedModel.capabilities.clicks;
                }
            });
        }
    } else {
        const inputHtml = `
            <label for="third-party-model-id" class="form-label">Model ID</label>
            <input type="text" class="form-control" id="third-party-model-id" placeholder="e.g., gpt-4, claude-3-opus">
            <div class="form-text mb-3">Enter the model ID as recognized by the provider.</div>

            <div class="mb-3">
                <label for="custom-model-api-base" class="form-label">API Base</label>
                <input type="text" class="form-control" id="custom-model-api-base" placeholder="Enter API base for this model">
            </div>

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

            // If this is a predefined model with a default config, use those values
            const providerModels = PROVIDER_DEFAULT_CONFIGS[providerId] || [];
            const defaultModelConfig = providerModels.find(model => model.model_id === modelId);

            if (!hasPredefinedModels(providerId)) {
                const customApiBase = document.getElementById('custom-model-api-base').value.trim();
                const customApiKey = document.getElementById('custom-model-api-key').value.trim();
                const customNCtx = parseInt(document.getElementById('custom-model-n-ctx').value.trim(), 10);
                const customSupportsTools = document.getElementById('custom-model-supports-tools').checked;
                const customSupportsMultimodality = document.getElementById('custom-model-supports-multimodality').checked;
                const customTokenizerUri = document.getElementById('custom-model-tokenizer-uri').value.trim();

                // Validate required fields
                if (!customApiBase) {
                    const error_message = "API Base is required for custom model configuration";
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
                    api_base: customApiBase,
                    api_key: customApiKey,
                    n_ctx: customNCtx,
                    supports_tools: customSupportsTools,
                    supports_multimodality: customSupportsMultimodality,
                    tokenizer_uri: customTokenizerUri || null
                };
            } else if (defaultModelConfig) {
                // For predefined models, we can store additional default capabilities
                // from the model config if needed
                modelConfig.default_capabilities = {
                    tools: defaultModelConfig.capabilities.tools,
                    multimodal: defaultModelConfig.capabilities.multimodal,
                    completion: defaultModelConfig.capabilities.completion
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
    // Check if the provider exists
    if (!apiConfig.providers[providerId]) {
        general_error("Provider configuration not found");
        return;
    }

    // Check if the model exists and belongs to this provider
    const modelConfig = apiConfig.models[modelId];
    if (!modelConfig || modelConfig.provider_id !== providerId) {
        general_error("Model configuration not found");
        return;
    }

    const modelIdContainer = document.getElementById('add-third-party-model-modal-id-container');
    modelIdContainer.dataset.providerId = providerId;
    modelIdContainer.dataset.modelId = modelId;
    modelIdContainer.dataset.isEdit = 'true';

    // Check if this is a predefined model or a custom model
    const hasCustomConfig = modelConfig.api_base !== undefined;

    if (hasPredefinedModels(providerId)) {
        // For predefined models, we only show capability checkboxes
        const editHtml = `
            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-agentic" ${modelConfig.capabilities.agent ? 'checked' : ''}>
                <label class="form-check-label" for="third-party-model-supports-agentic">
                    Supports Agentic Mode
                </label>
                <div class="form-text">Enable if this model supports autonomous agent functionality.</div>
            </div>

            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-clicks" ${modelConfig.capabilities.clicks ? 'checked' : ''}>
                <label class="form-check-label" for="third-party-model-supports-clicks">
                    Supports Clicks
                </label>
                <div class="form-text">Enable if this model supports click interactions.</div>
            </div>
        `;

        modelIdContainer.innerHTML = editHtml;
        document.getElementById('add-third-party-model-modal-label').textContent = 'Edit Model';
    } else {
        // For custom models, show all configuration options
        const editHtml = `
            <div class="mb-3">
                <label for="custom-model-api-base" class="form-label">API Base</label>
                <input type="text" class="form-control" id="custom-model-api-base" value="${modelConfig.api_base || ''}" placeholder="Enter API base for this model">
            </div>

            <div class="mb-3">
                <label for="custom-model-api-key" class="form-label">API Key</label>
                <input type="text" class="form-control" id="custom-model-api-key" value="${modelConfig.api_key || ''}" placeholder="Enter API key for this model">
            </div>

            <div class="mb-3">
                <label for="custom-model-n-ctx" class="form-label">Context Size (n_ctx)</label>
                <input type="number" class="form-control" id="custom-model-n-ctx" value="${modelConfig.n_ctx || 8192}" placeholder="e.g., 8192" min="1024" step="1024">
                <div class="form-text">Maximum number of tokens the model can process.</div>
            </div>

            <div class="form-check mb-3">
                <input class="form-check-input" type="checkbox" id="custom-model-supports-tools" ${modelConfig.capabilities.tools ? 'checked' : ''}>
                <label class="form-check-label" for="custom-model-supports-tools">
                    Supports Tools
                </label>
                <div class="form-text">Enable if this model supports function calling/tools.</div>
            </div>

            <div class="form-check mb-3">
                <input class="form-check-input" type="checkbox" id="custom-model-supports-multimodality" ${modelConfig.capabilities.multimodal ? 'checked' : ''}>
                <label class="form-check-label" for="custom-model-supports-multimodality">
                    Supports Multimodality
                </label>
                <div class="form-text">Enable if this model supports images and other media types.</div>
            </div>

            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-agentic" ${modelConfig.capabilities.agent ? 'checked' : ''}>
                <label class="form-check-label" for="third-party-model-supports-agentic">
                    Supports Agentic Mode
                </label>
                <div class="form-text">Enable if this model supports autonomous agent functionality.</div>
            </div>

            <div class="form-check mb-2">
                <input class="form-check-input" type="checkbox" id="third-party-model-supports-clicks" ${modelConfig.capabilities.clicks ? 'checked' : ''}>
                <label class="form-check-label" for="third-party-model-supports-clicks">
                    Supports Clicks
                </label>
                <div class="form-text">Enable if this model supports click interactions.</div>
            </div>

            <div class="mb-3">
                <label for="custom-model-tokenizer-uri" class="form-label">Tokenizer URI (Optional)</label>
                <input type="text" class="form-control" id="custom-model-tokenizer-uri" value="${modelConfig.tokenizer_uri || ''}" placeholder="e.g., https://huggingface.co/model/tokenizer.json">
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

    // Check if the provider exists
    if (!apiConfig.providers[providerId]) {
        const error_message = "No provider in config, can't update model";
        console.error(error_message);
        general_error(error_message);
        return;
    }

    // Check if the model exists
    if (!apiConfig.models[modelId] || apiConfig.models[modelId].provider_id !== providerId) {
        const error_message = "No model in config, can't update model";
        console.error(error_message);
        general_error(error_message);
        return;
    }

    // Get the values of the capability checkboxes
    const supportsAgentic = document.getElementById('third-party-model-supports-agentic').checked;
    const supportsClicks = document.getElementById('third-party-model-supports-clicks').checked;

    // Update the model configuration
    const modelConfig = apiConfig.models[modelId];

    // Update capabilities
    modelConfig.capabilities.agent = supportsAgentic;
    modelConfig.capabilities.clicks = supportsClicks;

    if (hasPredefinedModels(providerId)) {
        // For predefined models, we only update the capabilities
        // Preserve other properties from the default configuration
        const providerModels = PROVIDER_DEFAULT_CONFIGS[providerId] || [];
        const defaultConfig = providerModels.find(m => m.model_id === modelId);

        if (defaultConfig) {
            // Keep default capabilities but update agent and clicks
            modelConfig.capabilities.tools = defaultConfig.capabilities.tools;
            modelConfig.capabilities.multimodal = defaultConfig.capabilities.multimodal;
            modelConfig.capabilities.completion = defaultConfig.capabilities.completion;
        }

        // Remove custom properties if they exist
        delete modelConfig.api_base;
        delete modelConfig.api_key;
        delete modelConfig.tokenizer_uri;
    } else {
        // For custom models, update all properties
        const customApiBase = document.getElementById('custom-model-api-base').value.trim();
        const customApiKey = document.getElementById('custom-model-api-key').value.trim();
        const customNCtx = parseInt(document.getElementById('custom-model-n-ctx').value.trim(), 10);
        const customSupportsTools = document.getElementById('custom-model-supports-tools').checked;
        const customSupportsMultimodality = document.getElementById('custom-model-supports-multimodality').checked;
        const customTokenizerUri = document.getElementById('custom-model-tokenizer-uri').value.trim();

        // Validate required fields
        if (!customApiBase) {
            const error_message = "API Base URL is required for custom model configuration";
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

        // Update model properties
        modelConfig.api_base = customApiBase;
        modelConfig.api_key = customApiKey;
        modelConfig.n_ctx = customNCtx;
        modelConfig.capabilities.tools = customSupportsTools;
        modelConfig.capabilities.multimodal = customSupportsMultimodality;

        // Update tokenizer URI
        if (customTokenizerUri) {
            modelConfig.tokenizer_uri = customTokenizerUri;
        } else {
            delete modelConfig.tokenizer_uri;
        }
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
    // Check if the model exists and belongs to the specified provider
    if (apiConfig.models[modelId] && apiConfig.models[modelId].provider_id === providerId) {
        // Remove the model from the models dictionary
        delete apiConfig.models[modelId];

        // Update the configuration
        updateConfiguration();

        // Update the UI
        updateUI();

        showSuccessToast("Model removed successfully");
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