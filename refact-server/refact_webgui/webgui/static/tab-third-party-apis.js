// Third-party API providers management
import { general_error } from './error.js';
let show_toast = false;

function initExtraHeadersUI() {
    const container = document.getElementById('extra-headers-container');
    container.innerHTML = '';

    const addButton = document.getElementById('add-extra-header-btn');
    addButton.addEventListener('click', function() {
        addExtraHeaderRow();
    });
}

// Function to add a new extra header row
function addExtraHeaderRow(key = '', value = '') {
    const container = document.getElementById('extra-headers-container');
    const headerIndex = container.children.length;

    const headerRow = document.createElement('div');
    headerRow.className = 'extra-header-row row mb-2';
    headerRow.dataset.index = headerIndex;

    headerRow.innerHTML = `
        <div class="col-5">
            <input type="text" class="form-control extra-header-key"
                   placeholder="Header Name" value="${key}" data-index="${headerIndex}">
        </div>
        <div class="col-5">
            <input type="text" class="form-control extra-header-value"
                   placeholder="Header Value" value="${value}" data-index="${headerIndex}">
        </div>
        <div class="col-2 text-center">
            <button type="button" class="btn btn-danger remove-header-btn" data-index="${headerIndex}">
                <i class="bi bi-trash"></i>
            </button>
        </div>
    `;

    container.appendChild(headerRow);

    const removeButton = headerRow.querySelector('.remove-header-btn');
    removeButton.addEventListener('click', function() {
        removeExtraHeaderRow(this.dataset.index);
    });
}

function removeExtraHeaderRow(index) {
    const container = document.getElementById('extra-headers-container');
    const rowToRemove = container.querySelector(`.extra-header-row[data-index="${index}"]`);
    if (rowToRemove) {
        container.removeChild(rowToRemove);
    }
}

function collectExtraHeaders() {
    const headers = {};
    const container = document.getElementById('extra-headers-container');

    if (!container) {
        return headers;
    }

    const rows = container.querySelectorAll('.extra-header-row');
    if (rows.length === 0) {
        return headers;
    }

    rows.forEach((row, index) => {
        const keyInput = row.querySelector('.extra-header-key');
        const valueInput = row.querySelector('.extra-header-value');

        if (!keyInput || !valueInput) {
            console.log(`Inputs not found for row ${index}`);
            return;
        }

        const key = keyInput.value.trim();
        const value = valueInput.value.trim();
        if (key && value) {
            headers[key] = value;
        }
    });

    return headers;
}

// Provider default configurations with their available models
// This will be populated from litellm
let PROVIDER_DEFAULT_CONFIGS = {};

// Tokenizer upload modal
let tokenizer_upload_modal;

// Store the configuration
let apiConfig = {
    providers: {},
    models: {}
};

// Track expanded/collapsed state of providers and tokenizer sections
let expandedProviders = {};
let expandedTokenizerSections = {
    defaults: false,
    uploaded: false,
};

const modelNamePattern = /^[a-zA-Z0-9_\.\-\/]+$/;

export async function init(general_error) {
    let req = await fetch('/tab-third-party-apis.html');
    document.querySelector('#third-party-apis').innerHTML = await req.text();

    loadProvidersFromLiteLLM();
    loadConfiguration();
    initializeProvidersList();
    loadTokenizers();

    // Initialize the tokenizer upload modal after the HTML is loaded
    setTimeout(() => {
        initializeTokenizerModals();
    }, 100);
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
            <div class="models-list" id="${providerId}-models-list">
                <!-- Enabled models will be populated here when configuration is loaded -->
                <div class="alert alert-info" id="${providerId}-no-enabled-models-msg">
                    No models enabled for this provider. Use the "Add Model" button below to add and enable models.
                </div>
            </div>
        `;

        providerCard.innerHTML = `
            <div class="card-header d-flex justify-content-between align-items-center provider-header" data-provider="${providerId}">
                <h5 class="mb-0 provider-title" data-provider="${providerId}">
                    ${providerId}
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
            updateConfiguration();
        });
    });

    document.querySelectorAll('.provider-header, .provider-title').forEach(header => {
        header.addEventListener('click', function(event) {
            if (event.target.classList.contains('provider-toggle') || 
                event.target.classList.contains('remove-provider-btn') ||
                event.target.closest('.remove-provider-btn') ||
                event.target.closest('.form-check')) {
                return;
            }

            const providerId = this.dataset.provider;
            const providerBody = document.getElementById(`${providerId}-body`);

            const isVisible = providerBody.style.display !== 'none';
            const newExpandedState = !isVisible;

            setProviderCollapsedState(providerId, newExpandedState);

            expandedProviders[providerId] = newExpandedState;
        });
    });

    document.querySelectorAll('.remove-provider-btn').forEach(button => {
        button.addEventListener('click', function() {
            const providerId = this.dataset.provider;
            if (confirm(`Are you sure you want to remove the ${providerId} provider?`)) {
                delete apiConfig.providers[providerId];
                delete expandedProviders[providerId];
                const updatedModels = {};
                Object.entries(apiConfig.models).forEach(([modelId, modelConfig]) => {
                    if (Object.keys(apiConfig.providers).includes(modelConfig.provider_id)) {
                        updatedModels[modelId] = modelConfig;
                    }
                });
                apiConfig.models = updatedModels;
                saveConfiguration();
                initializeProvidersList();
                updateUI();
                showSuccessToast("Provider removed successfully");
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
    Object.keys(apiConfig.providers).forEach(providerId => {
        const toggle = document.getElementById(`${providerId}-toggle`);
        if (toggle) {
            apiConfig.providers[providerId].enabled = toggle.checked;
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

function maskApiKey(apiKey) {
    const apiKeyMask = "****";
    return apiKey.length > 16
        ? apiKey.substring(0, 4) + apiKeyMask + apiKey.substring(apiKey.length - 4)
        : "****" + apiKeyMask + "****";
}

function providerApiKeys(providerId) {
    const apiKeys = Object.entries(apiConfig.models)
        .filter(([_, modelConfig]) => modelConfig.provider_id === providerId)
        .filter(([_, modelConfig]) => !!modelConfig.api_key)
        .map(([_, modelConfig]) => modelConfig.api_key);
    return [...new Set(apiKeys)].sort();
}

function updateUI() {
    // First, uncheck all toggles and reset provider displays
    document.querySelectorAll('.provider-toggle').forEach(toggle => {
        toggle.checked = false;
        const providerId = toggle.dataset.provider;
        document.getElementById(`${providerId}-body`).style.display = 'none';
    });

    // Update UI based on configuration
    Object.entries(apiConfig.providers).forEach(([providerId, providerConfig]) => {
        const isEnabled = providerConfig.enabled !== undefined ? providerConfig.enabled : true;

        const toggle = document.getElementById(`${providerId}-toggle`);
        if (toggle) {
            toggle.checked = isEnabled;

            const isExpanded = expandedProviders[providerId] !== undefined ? expandedProviders[providerId] : false;
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

                        // Ensure modelConfig has capabilities
                        if (!modelConfig.capabilities) {
                            modelConfig.capabilities = {
                                tools: defaultCapabilities && defaultCapabilities.tools,
                                multimodal: defaultCapabilities && defaultCapabilities.multimodal,
                                agent: defaultCapabilities && defaultCapabilities.agent,
                                clicks: defaultCapabilities && defaultCapabilities.clicks,
                                completion: defaultCapabilities && defaultCapabilities.completion,
                            };
                        }

                        let capabilitiesBadges = '';
                        if (modelConfig.capabilities.agent) {
                            capabilitiesBadges += '<span class="badge bg-info me-1" title="Supports Agentic Mode">Agent</span>';
                        }
                        if (modelConfig.capabilities.clicks) {
                            capabilitiesBadges += '<span class="badge bg-success me-1" title="Supports Click Interactions">Clicks</span>';
                        }
                        if (modelConfig.capabilities.tools) {
                            capabilitiesBadges += '<span class="badge bg-secondary me-1" title="Supports Function Calling/Tools">Tools</span>';
                        }
                        if (modelConfig.capabilities.multimodal) {
                            capabilitiesBadges += '<span class="badge bg-primary me-1" title="Supports Images and Other Media">Multimodal</span>';
                        }
                        if (modelConfig.capabilities.reasoning) {
                            const reasoningType = modelConfig.capabilities.reasoning;
                            capabilitiesBadges += `<span class="badge bg-warning me-1" title="Supports ${reasoningType} Reasoning">Reasoning: ${reasoningType}</span>`;
                        }
                        if (modelConfig.capabilities.boost_reasoning) {
                            capabilitiesBadges += '<span class="badge bg-warning-subtle text-dark me-1" title="Boost Reasoning Enabled">Boost</span>';
                        }

                        // Force refresh of badges by adding a timestamp to ensure DOM updates
                        capabilitiesBadges += `<span class="d-none">${Date.now()}</span>`;

                        const modelItem = document.createElement('div');
                        modelItem.className = 'enabled-model-item mb-2 d-flex justify-content-between align-items-center';

                        // Display model_name (which is modelId in the UI) and actual model_id for inference
                        const modelName = modelId; // This is actually the model_name on the server
                        const actualModelId = modelConfig.model_id; // This is the actual model_id for litellm inference

                        let modelDisplayName = modelName;
                        if (modelName !== actualModelId) {
                            modelDisplayName = `${modelName} <span class="text-muted small">(${actualModelId})</span>`;
                        }

                        modelItem.innerHTML = `
                                <div class="d-flex align-items-center model-info" data-provider="${providerId}" data-model="${modelId}">
                                    <span class="model-name">${modelDisplayName}</span>
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

    Object.keys(PROVIDER_DEFAULT_CONFIGS).forEach((providerId) => {
        const option = document.createElement('option');
        option.value = providerId;
        option.textContent = providerId;
        option.dataset.name = providerId;
        providerIdSelect.appendChild(option);
    });

    const modal = new bootstrap.Modal(document.getElementById('add-third-party-provider-modal'));
    modal.show();

    document.getElementById('add-third-party-provider-submit').onclick = function() {
        addProvider();
    };
}

function addProvider() {
    const providerId = document.getElementById('third-party-provider-id').value.trim().toLowerCase();

    // Check if the selected provider requires an API key
    const providerSelect = document.getElementById('third-party-provider-id');
    const selectedOption = providerSelect.options[providerSelect.selectedIndex];

    if (!providerId) {
        const error_message = "Provider ID is required"
        console.error(error_message);
        general_error(error_message);
        return;
    }

    apiConfig.providers[providerId] = {
        enabled: true
    };

    saveConfiguration();
    initializeProvidersList();
    updateUI();

    const modal = bootstrap.Modal.getInstance(document.getElementById('add-third-party-provider-modal'));
    modal.hide();

    showSuccessToast("Provider added successfully");
}

function dependentCheckboxSet(elementId, dependentElementId) {
    const element = document.getElementById(elementId);
    const dependent = document.getElementById(dependentElementId);
    if (element.checked) {
        dependent.disabled = false;
    } else {
        dependent.checked = false;
        dependent.disabled = true;
    }
}

function dependentCheckboxListener(elementId, dependentElementId) {
    const element = document.getElementById(elementId);
    element.addEventListener('change', function() {
        dependentCheckboxSet(elementId, dependentElementId);
    });
}

function showAddModelModal(providerId) {
    const modelIdContainer = document.getElementById('add-third-party-model-modal-id-container');
    modelIdContainer.dataset.providerId = providerId;

    // Reset edit mode flag
    modelIdContainer.dataset.isEdit = 'false';
    modelIdContainer.dataset.modelId = '';

    // Load available tokenizers for the dropdown
    loadTokenizersForDropdown();

    // Determine if we have predefined models for this provider
    const hasPredefined = hasPredefinedModels(providerId);
    const providerModels = hasPredefined ? PROVIDER_DEFAULT_CONFIGS[providerId] : [];

    // Create the model ID selection part - either dropdown or text input
    let modelIdSelectionHtml = '';
    if (hasPredefined) {
        modelIdSelectionHtml = `
            <label for="third-party-model-id" class="form-label">Model Name</label>
            <select class="form-select" id="third-party-model-id">
                <option value="" selected>-- Select a model --</option>
                ${providerModels.map(model => `<option value="${model.model_id}">${model.model_id}</option>`).join('')}
            </select>
            <div class="form-text mb-3">Select from available models for this provider.</div>
        `;
    } else {
        modelIdSelectionHtml = `
            <label for="third-party-model-id" class="form-label">Model Name</label>
            <input type="text" class="form-control" id="third-party-model-id" placeholder="e.g., My GPT-4, My Claude">
            <div class="form-text mb-3">Enter a name for this model (used for display purposes).</div>

            <label for="third-party-actual-model-id" class="form-label">Actual Model ID</label>
            <input type="text" class="form-control" id="third-party-actual-model-id" placeholder="e.g., gpt-4, claude-3-opus">
            <div class="form-text mb-3">Enter the model ID as recognized by the provider for inference.</div>
        `;
    }

    // Create advanced configuration form
    const modelConfigAdvancedHtml = `
        <div class="card mb-3">
            <div class="card-header">
                <h6 class="mb-0">Context and Token Limits</h6>
            </div>
            <div class="card-body">
                <div class="row">
                    <div class="col-md-6">
                        <div class="mb-3">
                            <label for="custom-model-n-ctx" class="form-label">Context Size (n_ctx)</label>
                            <input type="number" class="form-control" id="custom-model-n-ctx" placeholder="e.g., 8192" min="1024" step="1024" value="8192">
                            <div class="form-text">Maximum Context size of the model.</div>
                        </div>
                    </div>
                    <div class="col-md-6">
                        <div class="mb-3">
                            <label for="custom-model-max-tokens" class="form-label">Max Tokens (n_ctx)</label>
                            <input type="number" class="form-control" id="custom-model-max-tokens" placeholder="e.g., 4096" min="1024" step="1024" value="4096">
                            <div class="form-text">Maximum number of tokens the model can generate.</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <div class="card mb-3">
            <div class="card-header">
                <h6 class="mb-0">Model Capabilities</h6>
            </div>
            <div class="card-body">
                <div class="row">
                    <div class="col-md-6">
                        <div class="form-check mb-3">
                            <input class="form-check-input" type="checkbox" id="custom-model-supports-tools">
                            <label class="form-check-label" for="custom-model-supports-tools">
                                Supports Tools
                            </label>
                            <div class="form-text">Enable if this model supports function calling/tools.</div>
                        </div>
                        <div class="form-check mb-3">
                            <input class="form-check-input" type="checkbox" id="third-party-model-supports-agentic">
                            <label class="form-check-label" for="third-party-model-supports-agentic">
                                Supports Agentic Mode
                            </label>
                            <div class="form-text">Enable if this model supports autonomous agent functionality.</div>
                        </div>
                        <div class="mb-3">
                            <label for="third-party-model-reasoning-type" class="form-label">Reasoning Type</label>
                            <select class="form-select" id="third-party-model-reasoning-type">
                                <option value="">None</option>
                                <option value="openai">OpenAI</option>
                                <option value="anthropic">Anthropic</option>
                                <option value="deepseek">DeepSeek</option>
                            </select>
                            <div class="form-text">Select the reasoning type supported by this model.</div>
                        </div>
                    </div>
                    <div class="col-md-6">
                        <div class="form-check mb-3">
                            <input class="form-check-input" type="checkbox" id="custom-model-supports-multimodality">
                            <label class="form-check-label" for="custom-model-supports-multimodality">
                                Supports Multimodality
                            </label>
                            <div class="form-text">Enable if this model supports images and other media types.</div>
                        </div>
                        <div class="form-check mb-3">
                            <input class="form-check-input" type="checkbox" id="third-party-model-supports-clicks">
                            <label class="form-check-label" for="third-party-model-supports-clicks">
                                Supports Clicks
                            </label>
                            <div class="form-text">Enable if this model supports click interactions.</div>
                        </div>
                        <div class="form-check mb-3">
                            <input class="form-check-input" type="checkbox" id="third-party-model-boost-reasoning" disabled>
                            <label class="form-check-label" for="third-party-model-boost-reasoning">
                                Boost Reasoning
                            </label>
                            <div class="form-text">Enable to boost reasoning capabilities (requires reasoning type).</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <div class="card mb-3">
            <div class="card-header">
                <h6 class="mb-0">Extra Headers</h6>
            </div>
            <div class="card-body">
                <div class="mb-3">
                    <p class="form-text mb-2">Add optional HTTP headers to be sent with requests to this model's API.</p>
                    <div id="extra-headers-container">
                        <!-- Headers will be added here dynamically -->
                    </div>
                    <button type="button" class="btn btn-primary mt-2" id="add-extra-header-btn">
                        <i class="bi bi-plus"></i> Add Header
                    </button>
                </div>
            </div>
        </div>
    `;

    // Create the unified model configuration form
    const modelConfigHtml = `
        ${modelIdSelectionHtml}

        <div class="mb-3">
            <label for="custom-model-api-key" class="form-label">API Key</label>
            <select class="form-select" id="custom-model-api-key"></select>
            <div id="custom-model-api-key-input-container" style="display: none; margin-top: 10px;">
                <input type="text" class="form-control" id="custom-model-api-key-input" placeholder="Enter custom API key">
            </div>
        </div>

        <div class="mb-3" id="custom-model-api-base-container">
            <label for="custom-model-api-base" class="form-label">API Base</label>
            <input type="text" class="form-control" id="custom-model-api-base" placeholder="Enter API base for this model, ex. http://localhost:8888/v1">
        </div>

        <div class="mb-3">
            <label for="custom-model-tokenizer-id" class="form-label">Tokenizer</label>
            <div class="dropdown">
                <button class="btn btn-outline-secondary dropdown-toggle form-control text-start" type="button" id="tokenizer-dropdown-btn" data-bs-toggle="dropdown" aria-expanded="false">
                    Default (None)
                </button>
                <ul class="dropdown-menu w-100" id="tokenizer-dropdown-menu">
                    <li><a class="dropdown-item" href="#" data-value="">Default (None)</a></li>
                    <li><hr class="dropdown-divider"></li>
                    <!-- Custom tokenizers will be populated here -->
                </ul>
                <input type="hidden" id="custom-model-tokenizer-id" value="">
            </div>
            <div class="form-text">Tokenizer for this model. Leave as default or select from available tokenizers.</div>
        </div>

        <div role="button" id="advancedOptionsCollapseButton" data-bs-toggle="collapse" data-bs-target="#advancedOptionsCollapse" aria-expanded="false" aria-controls="advancedOptionsCollapse" class="mb-3">
            <div class="d-flex justify-content-between align-items-center">
                <h6 class="mb-0">Advanced Options</h6>
                <i class="bi bi-chevron-down"></i>
            </div>
        </div>
        <div class="mb-4" id="advancedOptionsCollapse">
            <div id="model-config-advanced-container">
                ${modelConfigAdvancedHtml}
            </div>
        </div>
    `;

    modelIdContainer.innerHTML = modelConfigHtml;

    const apiKeySelect = document.getElementById('custom-model-api-key');
    let selectedApiKey = !hasPredefined;
    providerApiKeys(providerId).forEach((apiKey) => {
        if (apiKey && apiKey.trim()) {
            const option = document.createElement('option');
            option.value = apiKey;
            option.textContent = maskApiKey(apiKey);
            if (!selectedApiKey) {
                option.selected = true;
                selectedApiKey = true;
            }
            apiKeySelect.appendChild(option);
        }
    });

    const customOption = document.createElement('option');
    customOption.value = 'custom';
    customOption.textContent = '-- Enter custom API key --';
    customOption.selected = !hasPredefined || !selectedApiKey;
    if (customOption.selected) {
        const customInputContainer = document.getElementById('custom-model-api-key-input-container');
        customInputContainer.style.display = 'block';
        document.getElementById('custom-model-api-key-input').value = '';
        document.getElementById('custom-model-api-key-input').focus();
    }
    apiKeySelect.appendChild(customOption);

    apiKeySelect.addEventListener('change', function() {
        const customInputContainer = document.getElementById('custom-model-api-key-input-container');
        if (this.value === 'custom') {
            customInputContainer.style.display = 'block';
            document.getElementById('custom-model-api-key-input').value = '';
            document.getElementById('custom-model-api-key-input').focus();
        } else {
            customInputContainer.style.display = 'none';
        }
    });

    // Collapse advanced block setup
    const collapseButtonElement = document.getElementById('advancedOptionsCollapseButton');
    const collapseElement = document.getElementById('advancedOptionsCollapse');
    collapseElement.classList.add("collapse");
    if (hasPredefined) {
        collapseButtonElement.setAttribute("aria-expanded", "false");
    } else {
        collapseButtonElement.setAttribute("aria-expanded", "true");
        collapseElement.classList.add("show");
    }

    const apiBaseContainer = document.getElementById('custom-model-api-base-container');
    apiBaseContainer.style.display = hasPredefined ? 'none' : '';

    initExtraHeadersUI();

    const modelSelect = document.getElementById('third-party-model-id');
    if (modelSelect && hasPredefined) {
        modelSelect.addEventListener('change', function() {
            const selectedModelId = this.value;
            const selectedModel = providerModels.find(model => model.model_id === selectedModelId);

            if (selectedModel) {
                // Pre-fill all capabilities and settings from the default model config
                document.getElementById('third-party-model-supports-agentic').checked =
                    selectedModel.capabilities.agent;
                document.getElementById('third-party-model-supports-clicks').checked =
                    selectedModel.capabilities.clicks;
                document.getElementById('custom-model-supports-tools').checked =
                    selectedModel.capabilities.tools;
                document.getElementById('custom-model-supports-multimodality').checked =
                    selectedModel.capabilities.multimodal;
                document.getElementById('custom-model-n-ctx').value =
                    selectedModel.n_ctx;
                document.getElementById('custom-model-max-tokens').value =
                    selectedModel.max_tokens;

                if (selectedModel.capabilities.reasoning) {
                    document.getElementById('third-party-model-reasoning-type').value =
                        selectedModel.capabilities.reasoning;
                    document.getElementById('third-party-model-boost-reasoning').disabled = false;
                    document.getElementById('third-party-model-boost-reasoning').checked =
                        selectedModel.capabilities.boost_reasoning || false;
                } else {
                    document.getElementById('third-party-model-reasoning-type').value = '';
                    document.getElementById('third-party-model-boost-reasoning').disabled = true;
                    document.getElementById('third-party-model-boost-reasoning').checked = false;
                }

                if (selectedModel.tokenizer_id) {
                    document.getElementById('custom-model-tokenizer-id').value = selectedModel.tokenizer_id;
                    const dropdownBtn = document.getElementById('tokenizer-dropdown-btn');
                    if (dropdownBtn) {
                        dropdownBtn.textContent = selectedModel.tokenizer_id;
                    }
                    const dropdownMenu = document.getElementById('tokenizer-dropdown-menu');
                    if (dropdownMenu) {
                        dropdownMenu.querySelectorAll('.dropdown-item').forEach(item => {
                            item.classList.remove('active');
                            if (item.getAttribute('data-value') === selectedModel.tokenizer_id) {
                                item.classList.add('active');
                            }
                        });
                    }
                }
            }

            dependentCheckboxSet('custom-model-supports-tools', 'third-party-model-supports-agentic');
            dependentCheckboxSet('custom-model-supports-multimodality', 'third-party-model-supports-clicks');
        });
    }

    dependentCheckboxSet('custom-model-supports-tools', 'third-party-model-supports-agentic');
    dependentCheckboxListener('custom-model-supports-tools', 'third-party-model-supports-agentic');
    dependentCheckboxSet('custom-model-supports-multimodality', 'third-party-model-supports-clicks');
    dependentCheckboxListener('custom-model-supports-multimodality', 'third-party-model-supports-clicks');

    const reasoningTypeSelect = document.getElementById('third-party-model-reasoning-type');
    reasoningTypeSelect.addEventListener('change', function() {
        const boostReasoningCheckbox = document.getElementById('third-party-model-boost-reasoning');
        if (this.value) {
            boostReasoningCheckbox.disabled = false;
        } else {
            boostReasoningCheckbox.checked = false;
            boostReasoningCheckbox.disabled = true;
        }
    });

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
    // Check if we're in edit mode
    const modelIdContainer = document.getElementById('add-third-party-model-modal-id-container');
    const isEdit = modelIdContainer.dataset.isEdit === 'true';

    // If we're in edit mode, call updateModel instead
    if (isEdit) {
        updateModel();
        return;
    }

    let reasoningType = document.getElementById('third-party-model-reasoning-type').value.trim();
    let boostReasoning = document.getElementById('third-party-model-boost-reasoning').checked;

    const extraHeaders = collectExtraHeaders();

    if (boostReasoning && !reasoningType) {
        const error_message = "Boost reasoning requires a reasoning type to be selected";
        console.error(error_message);
        general_error(error_message);
        return;
    }

    // Get the model ID from either the input field or the select dropdown
    let modelId;
    let actualModelId;
    const providerId = modelIdContainer.dataset.providerId;
    const modelIdElement = document.getElementById('third-party-model-id');
    const actualModelIdElement = document.getElementById('third-party-actual-model-id');

    if (!modelIdElement) {
        return;
    }

    // Get the model ID value (which is actually the model_name)
    modelId = modelIdElement.value.trim();

    if (!modelId) {
        const error_message = "Model Name is required";
        console.error(error_message);
        general_error(error_message);
        return;
    }

    const hasPredefined = hasPredefinedModels(providerId);

    // Validate model name pattern for custom providers
    if (!hasPredefined) {
        if (!modelNamePattern.test(modelId)) {
            const error_message = "Model Name can only contain letters, numbers, underscores, dots, and hyphens";
            console.error(error_message);
            general_error(error_message);
            return;
        }
    }

    // For predefined models, the actual model ID is the same as the model name
    // For custom models, get the actual model ID from the separate field
    if (hasPredefined) {
        actualModelId = modelId;
    } else {
        if (!actualModelIdElement) {
            return;
        }
        actualModelId = actualModelIdElement.value.trim();
        if (!actualModelId) {
            const error_message = "Actual Model ID is required";
            console.error(error_message);
            general_error(error_message);
            return;
        }
    }

    // Find the provider in the configuration
    const providerConfig = apiConfig.providers[providerId];
    if (providerConfig) {
        // Check if the model is already enabled for this provider
        const modelExists = Object.entries(apiConfig.models)
            .some(([existingModelId, model]) =>
                model.provider_id === providerId && existingModelId === modelId
            );

        if (!modelExists) {
            // Get user-specified values from the form
            const supportsAgentic = document.getElementById('third-party-model-supports-agentic').checked;
            const supportsClicks = document.getElementById('third-party-model-supports-clicks').checked;
            const supportsTools = document.getElementById('custom-model-supports-tools').checked;
            const supportsMultimodality = document.getElementById('custom-model-supports-multimodality').checked;
            reasoningType = document.getElementById('third-party-model-reasoning-type').value.trim();
            boostReasoning = document.getElementById('third-party-model-boost-reasoning').checked;
            const customApiBase = document.getElementById('custom-model-api-base').value.trim();

            // Get API key from either dropdown or custom input
            let customApiKey = '';
            const apiKeySelect = document.getElementById('custom-model-api-key');
            if (apiKeySelect.value === 'custom') {
                customApiKey = document.getElementById('custom-model-api-key-input').value.trim();
                if (!customApiKey) {
                    const error_message = "Custom API key is required when selecting the custom option";
                    console.error(error_message);
                    general_error(error_message);
                    return;
                }
            } else {
                customApiKey = apiKeySelect.value.trim();
            }

            const customNCtx = parseInt(document.getElementById('custom-model-n-ctx').value.trim(), 10);
            const customMaxTokens = parseInt(document.getElementById('custom-model-max-tokens').value.trim(), 10);
            const customTokenizerId = document.getElementById('custom-model-tokenizer-id').value.trim();

            // Validate context size
            if (isNaN(customNCtx) || customNCtx < 1024) {
                const error_message = "Context size must be a valid number greater than or equal to 1024";
                console.error(error_message);
                general_error(error_message);
                return;
            }

            // Find default model config if available
            const providerModels = PROVIDER_DEFAULT_CONFIGS[providerId] || [];
            const defaultModelConfig = providerModels.find(model => model.model_id === modelId);

            // Create a new model config with capabilities
            const modelConfig = {
                model_id: actualModelId, // Use the actual model ID for inference
                provider_id: providerId,
                api_base: customApiBase,
                api_key: customApiKey,
                n_ctx: customNCtx,
                max_tokens: customMaxTokens,
                extra_headers: Object.keys(extraHeaders).length > 0 ? extraHeaders : {},
                capabilities: {
                    agent: !!supportsAgentic,
                    clicks: !!supportsClicks,
                    tools: !!supportsTools,
                    multimodal: !!supportsMultimodality,
                    completion: !!(defaultModelConfig ? defaultModelConfig.capabilities.completion : false),
                    reasoning: reasoningType || null,
                    boost_reasoning: !!boostReasoning
                }
            };

            // Add tokenizer ID if provided
            if (customTokenizerId) {
                modelConfig.tokenizer_id = customTokenizerId;
            } else {
                modelConfig.tokenizer_id = null;
            }

            // Add the model config to the models dictionary
            apiConfig.models[modelId] = modelConfig;

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

    // First, call the same function that shows the modal for adding a model
    // This builds the default form
    showAddModelModal(providerId);

    // Set a flag so we know we are in edit mode
    const modelIdContainer = document.getElementById('add-third-party-model-modal-id-container');
    modelIdContainer.dataset.providerId = providerId;
    modelIdContainer.dataset.modelId = modelId;
    modelIdContainer.dataset.isEdit = 'true';

    if (modelConfig.extra_headers && Object.keys(modelConfig.extra_headers).length > 0) {
        const container = document.getElementById('extra-headers-container');
        container.innerHTML = '';
        Object.entries(modelConfig.extra_headers).forEach(([key, value]) => {
            addExtraHeaderRow(key, value);
        });
    }

    // Now pre-populate the fields with the data from the existing model configuration
    const modelIdElement = document.getElementById('third-party-model-id');
    if (modelIdElement) {
        // Display model_name (which is modelId in the UI)
        modelIdElement.value = modelId;

        // Disable the model id field - we do not allow changing a model's id
        modelIdElement.disabled = true;

        // For custom models, populate the actual model ID field
        const actualModelIdElement = document.getElementById('third-party-actual-model-id');
        if (actualModelIdElement && !hasPredefinedModels(providerId)) {
            actualModelIdElement.value = modelConfig.model_id;
        } else if (modelId !== modelConfig.model_id) {
            // For predefined models, add a note about the actual model_id if it's different
            const modelIdNote = document.createElement('div');
            modelIdNote.className = 'form-text text-info';
            modelIdNote.innerHTML = `<strong>Note:</strong> This model uses <code>${modelConfig.model_id}</code> as the actual model ID for inference.`;
            modelIdElement.parentNode.appendChild(modelIdNote);
        }
    }

    // Ensure capabilities object exists
    const capabilities = modelConfig.capabilities || {};

    // Fill in the form fields with the existing model data
    document.getElementById('custom-model-api-base').value = modelConfig.api_base || '';
    document.getElementById('custom-model-n-ctx').value = modelConfig.n_ctx || 8192;
    document.getElementById('custom-model-max-tokens').value = modelConfig.max_tokens || 4096;
    document.getElementById('custom-model-supports-tools').checked = capabilities.tools || false;
    document.getElementById('custom-model-supports-multimodality').checked = capabilities.multimodal || false;
    document.getElementById('third-party-model-supports-agentic').checked = capabilities.agent || false;
    document.getElementById('third-party-model-supports-clicks').checked = capabilities.clicks || false;

    const reasoningTypeSelect = document.getElementById('third-party-model-reasoning-type');
    reasoningTypeSelect.value = capabilities.reasoning || '';
    document.getElementById('third-party-model-boost-reasoning').checked = capabilities.boost_reasoning || false;

    if (capabilities.reasoning) {
        document.getElementById('third-party-model-boost-reasoning').disabled = false;
    } else {
        document.getElementById('third-party-model-boost-reasoning').disabled = true;
    }

    dependentCheckboxSet('custom-model-supports-tools', 'third-party-model-supports-agentic');
    dependentCheckboxSet('custom-model-supports-multimodality', 'third-party-model-supports-clicks');

    // Handle API key selection
    const apiKeySelect = document.getElementById('custom-model-api-key');
    const customApiKeyInputContainer = document.getElementById('custom-model-api-key-input-container');

    const keyIndex = providerApiKeys(providerId).indexOf(modelConfig.api_key);
    if (keyIndex >= 0) {
        apiKeySelect.value = modelConfig.api_key;
        customApiKeyInputContainer.style.display = 'none';
    } else {
        apiKeySelect.value = '';
        customApiKeyInputContainer.style.display = 'none';
    }

    const tokenizerIdElement = document.getElementById('custom-model-tokenizer-id');
    if (tokenizerIdElement) {
        tokenizerIdElement.value = modelConfig.tokenizer_id || '';
    }

    loadTokenizersForDropdown();

    const dropdownBtn = document.getElementById('tokenizer-dropdown-btn');
    if (modelConfig.tokenizer_id) {
        dropdownBtn.textContent = modelConfig.tokenizer_id;
    } else {
        dropdownBtn.textContent = 'Default (None)';
    }

    document.getElementById('add-third-party-model-modal-label').textContent = 'Edit Model';
    const submitBtn = document.getElementById('add-third-party-model-submit');
    submitBtn.textContent = 'Save Changes';

    submitBtn.onclick = function() {
        updateModel();
    };
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

    // Validate model name pattern for custom providers
    if (!hasPredefinedModels(providerId)) {
        if (!modelNamePattern.test(modelId)) {
            const error_message = "Model Name can only contain letters, numbers, underscores, dots, and hyphens";
            console.error(error_message);
            general_error(error_message);
            return;
        }
    }

    const extraHeaders = collectExtraHeaders();

    let reasoningType = document.getElementById('third-party-model-reasoning-type').value.trim();
    let boostReasoning = document.getElementById('third-party-model-boost-reasoning').checked;

    if (boostReasoning && !reasoningType) {
        const error_message = "Boost reasoning requires a reasoning type to be selected";
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

    // Get all values from the form
    const supportsAgentic = document.getElementById('third-party-model-supports-agentic').checked;
    const supportsClicks = document.getElementById('third-party-model-supports-clicks').checked;
    const supportsTools = document.getElementById('custom-model-supports-tools').checked;
    const supportsMultimodality = document.getElementById('custom-model-supports-multimodality').checked;
    reasoningType = document.getElementById('third-party-model-reasoning-type').value.trim();
    boostReasoning = document.getElementById('third-party-model-boost-reasoning').checked;
    const customApiBase = document.getElementById('custom-model-api-base').value.trim();

    // Get API key from either dropdown or custom input
    let customApiKey = '';
    const apiKeySelect = document.getElementById('custom-model-api-key');
    if (apiKeySelect.value === 'custom') {
        customApiKey = document.getElementById('custom-model-api-key-input').value.trim();
        if (!customApiKey) {
            const error_message = "Custom API key is required when selecting the custom option";
            console.error(error_message);
            general_error(error_message);
            return;
        }
    } else {
        customApiKey = apiKeySelect.value.trim();
    }

    const customNCtx = parseInt(document.getElementById('custom-model-n-ctx').value.trim(), 10);
    const customMaxTokens = parseInt(document.getElementById('custom-model-max-tokens').value.trim(), 10);
    const customTokenizerId = document.getElementById('custom-model-tokenizer-id').value.trim();

    // Validate context size
    if (isNaN(customNCtx) || customNCtx < 1024) {
        const error_message = "Context size must be a valid number greater than or equal to 1024";
        console.error(error_message);
        general_error(error_message);
        return;
    }

    // Get the current model configuration
    const modelConfig = apiConfig.models[modelId];

    // If we have a custom model with a separate actual model ID field, update it
    const actualModelIdElement = document.getElementById('third-party-actual-model-id');
    if (actualModelIdElement && !hasPredefinedModels(providerId)) {
        const newActualModelId = actualModelIdElement.value.trim();
        if (newActualModelId && newActualModelId !== modelConfig.model_id) {
            modelConfig.model_id = newActualModelId;
        }
    }

    modelConfig.capabilities.agent = supportsAgentic;
    modelConfig.capabilities.clicks = supportsClicks;
    modelConfig.capabilities.tools = supportsTools;
    modelConfig.capabilities.multimodal = supportsMultimodality;
    modelConfig.capabilities.reasoning = reasoningType || null;
    modelConfig.capabilities.boost_reasoning = boostReasoning;

    modelConfig.n_ctx = customNCtx;
    modelConfig.max_tokens = customMaxTokens;

    modelConfig.api_base = customApiBase ? customApiBase : null;
    modelConfig.api_key = customApiKey ? customApiKey : null;
    modelConfig.extra_headers = Object.keys(extraHeaders).length > 0 ? extraHeaders : {};

    // Update tokenizer ID if provided, otherwise set to null
    if (customTokenizerId) {
        modelConfig.tokenizer_id = customTokenizerId;
    } else {
        modelConfig.tokenizer_id = null;
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
        // Force a complete refresh of the configuration and UI
        loadProvidersFromLiteLLM();
        loadConfiguration();
        loadTokenizers();

        // Small delay to ensure data is loaded before initializing the UI
        setTimeout(() => {
            initializeProvidersList();
            updateUI();
        }, 100);
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

    initializeTokenizerModals();
}

export function tab_switched_away() {
    // Nothing to do when switching away
}

export function tab_update_each_couple_of_seconds() {
    // Nothing to update periodically
}

function loadTokenizers() {
    fetch("/tab-third-party-apis-get-tokenizers")
        .then(response => response.json())
        .then(data => {
            updateTokenizersList(data);
        })
        .catch(error => {
            console.error("Error loading tokenizers:", error);
            general_error(error);
        });
}

function loadTokenizersForDropdown() {
    const dropdownMenu = document.getElementById('tokenizer-dropdown-menu');
    const tokenizerId = document.getElementById('custom-model-tokenizer-id');

    if (!dropdownMenu || !tokenizerId) {
        return;
    }

    fetch("/tab-third-party-apis-get-tokenizers")
        .then(response => {
            if (!response.ok) {
                throw new Error(`Server returned ${response.status}: ${response.statusText}`);
            }
            return response.json();
        })
        .then(data => {
            if (data && (data.defaults || data.uploaded)) {
                const allTokenizers = {
                    defaults: data.defaults || [],
                    uploaded: data.uploaded || []
                };
                populateTokenizerDropdown(allTokenizers);
            } else {
                console.warn("Unexpected tokenizer data format:", data);
                populateTokenizerDropdown({ defaults: [], uploaded: [] });
            }
        })
        .catch(error => {
            console.error("Error loading tokenizers for dropdown:", error);
            populateTokenizerDropdown({ defaults: [], uploaded: [] });
        });
}

function populateTokenizerDropdown(tokenizers) {
    const dropdownMenu = document.getElementById('tokenizer-dropdown-menu');
    const tokenizerId = document.getElementById('custom-model-tokenizer-id');
    const currentValue = tokenizerId.value.trim();

    const defaults = tokenizers.defaults || [];
    const uploaded = tokenizers.uploaded || [];
    const hasDefaults = defaults.length > 0;
    const firstDefaultTokenizer = hasDefaults ? defaults[0] : '';

    if (!currentValue && firstDefaultTokenizer) {
        tokenizerId.value = firstDefaultTokenizer;
    }

    const updatedCurrentValue = tokenizerId.value.trim();

    let tokenizersHtml = '';
    if (!hasDefaults) {
        tokenizersHtml += `
            <li><a class="dropdown-item ${!updatedCurrentValue ? 'active' : ''}" href="#" data-value="">Default (None)</a></li>
            <li><hr class="dropdown-divider"></li>
        `;
    }

    if (hasDefaults) {
        tokenizersHtml += '<li><h6 class="dropdown-header">default</h6></li>';
        defaults.forEach(tokenizer => {
            const isActive = updatedCurrentValue === tokenizer;
            tokenizersHtml += `<li><a class="dropdown-item ${isActive ? 'active' : ''}" href="#" data-value="${tokenizer}">${tokenizer}</a></li>`;
        });
    }

    if (uploaded.length > 0) {
        if (hasDefaults) {
            tokenizersHtml += '<li><hr class="dropdown-divider"></li>';
        }
        tokenizersHtml += '<li><h6 class="dropdown-header">custom</h6></li>';
        uploaded.forEach(tokenizer => {
            const isActive = updatedCurrentValue === tokenizer;
            tokenizersHtml += `<li><a class="dropdown-item ${isActive ? 'active' : ''}" href="#" data-value="${tokenizer}">${tokenizer}</a></li>`;
        });
    } else if (hasDefaults) {
        tokenizersHtml += '<li><hr class="dropdown-divider"></li>';
        tokenizersHtml += '<li><a class="dropdown-item disabled" href="#">No custom tokenizers available</a></li>';
    }

    if (!hasDefaults && uploaded.length === 0) {
        tokenizersHtml += '<li><a class="dropdown-item disabled" href="#">No tokenizers available</a></li>';
    }

    dropdownMenu.innerHTML = tokenizersHtml;

    dropdownMenu.querySelectorAll('.dropdown-item:not(.disabled)').forEach(item => {
        item.addEventListener('click', function(e) {
            e.preventDefault();
            const selectedTokenizerId = this.getAttribute('data-value');
            document.getElementById('custom-model-tokenizer-id').value = selectedTokenizerId;

            const dropdownBtn = document.getElementById('tokenizer-dropdown-btn');
            if (selectedTokenizerId) {
                dropdownBtn.textContent = selectedTokenizerId;
            } else {
                dropdownBtn.textContent = 'Default (None)';
            }

            dropdownMenu.querySelectorAll('.dropdown-item').forEach(i => i.classList.remove('active'));
            this.classList.add('active');
        });
    });

    const dropdownBtn = document.getElementById('tokenizer-dropdown-btn');
    if (updatedCurrentValue) {
        dropdownBtn.textContent = updatedCurrentValue;
    } else {
        dropdownBtn.textContent = 'Default (None)';
    }
}

function updateTokenizersList(data) {
    const tokenizersContainer = document.getElementById('tokenizers-list');
    const noTokenizersMsg = document.getElementById('no-tokenizers-msg');

    tokenizersContainer.innerHTML = '';
    tokenizersContainer.appendChild(noTokenizersMsg);

    const defaults = data.defaults || [];
    const uploaded = data.uploaded || [];

    if (defaults.length === 0 && uploaded.length === 0) {
        noTokenizersMsg.style.display = 'block';
        return;
    }

    noTokenizersMsg.style.display = 'none';

    const defaultsCard = document.createElement('div');
    defaultsCard.className = 'card mb-3 api-provider-container';
    tokenizersContainer.appendChild(defaultsCard);

    const defaultsHeader = document.createElement('div');
    defaultsHeader.className = 'card-header d-flex justify-content-between align-items-center provider-header';
    if (!expandedTokenizerSections.defaults) {
        defaultsHeader.classList.add('collapsed');
    }
    defaultsHeader.innerHTML = `
        <h5 class="mb-0 provider-title">default</h5>
        <i class="bi bi-chevron-${expandedTokenizerSections.uploaded ? 'down' : 'right'}"></i>
    `;
    defaultsHeader.style.cursor = 'pointer';
    defaultsCard.appendChild(defaultsHeader);

    const defaultsBody = document.createElement('div');
    defaultsBody.className = 'card-body provider-body';
    defaultsBody.style.display = expandedTokenizerSections.defaults ? 'block' : 'none';
    defaultsCard.appendChild(defaultsBody);

    defaultsHeader.addEventListener('click', function() {
        const chevron = this.querySelector('.bi');
        const isCollapsed = this.classList.contains('collapsed');

        if (isCollapsed) {
            defaultsBody.style.display = 'block';
            chevron.className = 'bi bi-chevron-down';
            this.classList.remove('collapsed');
            expandedTokenizerSections.defaults = true;
        } else {
            defaultsBody.style.display = 'none';
            chevron.className = 'bi bi-chevron-right';
            this.classList.add('collapsed');
            expandedTokenizerSections.defaults = false;
        }
    });

    if (defaults.length > 0) {
        defaults.forEach(tokenizer_id => {
            const tokenizerItem = document.createElement('div');
            tokenizerItem.className = 'mb-2 enabled-model-item';
            tokenizerItem.textContent = tokenizer_id;
            defaultsBody.appendChild(tokenizerItem);
        });
    } else {
        const noDefaultsMsg = document.createElement('div');
        noDefaultsMsg.className = 'text-muted';
        noDefaultsMsg.textContent = 'No default tokenizers available';
        defaultsBody.appendChild(noDefaultsMsg);
    }

    const uploadedCard = document.createElement('div');
    uploadedCard.className = 'card mb-3 api-provider-container';
    tokenizersContainer.appendChild(uploadedCard);

    const uploadedHeader = document.createElement('div');
    uploadedHeader.className = 'card-header d-flex justify-content-between align-items-center provider-header';
    uploadedHeader.dataset.section = 'uploaded';
    if (!expandedTokenizerSections.uploaded) {
        uploadedHeader.classList.add('collapsed');
    }
    uploadedHeader.innerHTML = `
        <h5 class="mb-0 provider-title">custom</h5>
        <i class="bi bi-chevron-${expandedTokenizerSections.uploaded ? 'down' : 'right'}"></i>
    `;
    uploadedHeader.style.cursor = 'pointer';
    uploadedCard.appendChild(uploadedHeader);

    const uploadedBody = document.createElement('div');
    uploadedBody.className = 'card-body provider-body';
    uploadedBody.style.display = expandedTokenizerSections.uploaded ? 'block' : 'none';
    uploadedCard.appendChild(uploadedBody);

    uploadedHeader.addEventListener('click', function() {
        const chevron = this.querySelector('.bi');
        const isCollapsed = this.classList.contains('collapsed');

        if (isCollapsed) {
            uploadedBody.style.display = 'block';
            chevron.className = 'bi bi-chevron-down';
            this.classList.remove('collapsed');
            expandedTokenizerSections.uploaded = true;
        } else {
            uploadedBody.style.display = 'none';
            chevron.className = 'bi bi-chevron-right';
            this.classList.add('collapsed');
            expandedTokenizerSections.uploaded = false;
        }
    });

    if (uploaded.length > 0) {
        uploaded.forEach(tokenizer_id => {
            const tokenizerItem = document.createElement('div');
            tokenizerItem.className = 'd-flex justify-content-between align-items-center mb-2 enabled-model-item';
            tokenizerItem.innerHTML = `
                <span class="model-name">${tokenizer_id}</span>
                <button class="btn btn-sm btn-outline-danger delete-tokenizer-btn" data-tokenizer-id="${tokenizer_id}">
                    <i class="bi bi-trash"></i>
                </button>
            `;
            uploadedBody.appendChild(tokenizerItem);

            const deleteBtn = tokenizerItem.querySelector('.delete-tokenizer-btn');
            deleteBtn.addEventListener('click', function(e) {
                e.stopPropagation();
                deleteTokenizer(this.dataset.tokenizerId);
            });
        });
    } else {
        const noUploadedMsg = document.createElement('div');
        noUploadedMsg.className = 'text-muted';
        noUploadedMsg.textContent = 'No custom tokenizers uploaded';
        uploadedBody.appendChild(noUploadedMsg);
    }

    const uploadButtonContainer = document.createElement('div');
    uploadButtonContainer.className = 'card mb-3 add-provider-card';

    const uploadButtonDiv = document.createElement('div');
    uploadButtonDiv.className = 'card-body text-center py-3';
    uploadButtonDiv.innerHTML = `
        <button id="upload-tokenizer-btn" class="btn btn-primary">
            <i class="bi bi-plus-circle"></i> Upload Tokenizer
        </button>
    `;
    uploadButtonContainer.appendChild(uploadButtonDiv);
    tokenizersContainer.appendChild(uploadButtonContainer);

    document.getElementById('upload-tokenizer-btn').addEventListener('click', function() {
        document.getElementById('tokenizer-upload-form').reset();
        document.getElementById('tokenizer-upload-error').classList.add('d-none');
        tokenizer_upload_modal.show();
    });
}

function initializeTokenizerModals() {
    const tokenizer_upload_modal_element = document.getElementById('tokenizer-upload-modal');

    if (tokenizer_upload_modal_element._bsModal) {
        tokenizer_upload_modal = tokenizer_upload_modal_element._bsModal;
    } else {
        tokenizer_upload_modal = new bootstrap.Modal(tokenizer_upload_modal_element);
        tokenizer_upload_modal_element._bsModal = tokenizer_upload_modal;
    }

    const modelModal = document.getElementById('add-third-party-model-modal');
    if (modelModal) {
        modelModal.addEventListener('show.bs.modal', function() {
            loadTokenizersForDropdown();
        });
    }

    tokenizer_upload_modal_element.addEventListener('hidden.bs.modal', function () {
        const overlay = document.querySelector('.modal-overlay');
        if (overlay && overlay.parentNode) {
            overlay.parentNode.removeChild(overlay);
        }

        const allButtons = tokenizer_upload_modal_element.querySelectorAll('button');
        const allInputs = tokenizer_upload_modal_element.querySelectorAll('input');
        allButtons.forEach(btn => btn.disabled = false);
        allInputs.forEach(input => input.disabled = false);

        document.getElementById('tokenizer-upload-form').reset();
        document.getElementById('tokenizer-upload-error').classList.add('d-none');
    });

    document.getElementById('tokenizer-upload-submit').onclick = function() {
        uploadTokenizer();
    };
}

function uploadTokenizer() {
    const tokenizerId = document.getElementById('tokenizer-id').value.trim();
    const tokenizerFile = document.getElementById('tokenizer-file').files[0];
    const errorElement = document.getElementById('tokenizer-upload-error');
    const modalElement = document.getElementById('tokenizer-upload-modal');
    errorElement.classList.add('d-none');

    if (!tokenizerId) {
        errorElement.textContent = "Please enter a tokenizer ID";
        errorElement.classList.remove('d-none');
        return;
    }

    if (!tokenizerFile) {
        errorElement.textContent = "Please select a tokenizer file";
        errorElement.classList.remove('d-none');
        return;
    }

    if (!tokenizerFile.name.endsWith('.json')) {
        errorElement.textContent = "Tokenizer file must have a .json extension";
        errorElement.classList.remove('d-none');
        return;
    }

    const formData = new FormData();
    formData.append('tokenizer_id', tokenizerId);
    formData.append('file', tokenizerFile);

    const modalOverlay = document.createElement('div');
    modalOverlay.className = 'modal-overlay';
    modalOverlay.style.position = 'absolute';
    modalOverlay.style.top = '0';
    modalOverlay.style.left = '0';
    modalOverlay.style.width = '100%';
    modalOverlay.style.height = '100%';
    modalOverlay.style.backgroundColor = 'rgba(0,0,0,0.2)';
    modalOverlay.style.zIndex = '1050';
    modalOverlay.style.display = 'flex';
    modalOverlay.style.justifyContent = 'center';
    modalOverlay.style.alignItems = 'center';
    modalOverlay.innerHTML = '<div class="spinner-border text-primary" role="status"><span class="visually-hidden">Loading...</span></div>';

    const modalDialog = modalElement.querySelector('.modal-dialog');
    modalDialog.parentNode.insertBefore(modalOverlay, modalDialog);

    const allButtons = modalElement.querySelectorAll('button');
    const allInputs = modalElement.querySelectorAll('input');
    allButtons.forEach(btn => btn.disabled = true);
    allInputs.forEach(input => input.disabled = true);

    const submitButton = document.getElementById('tokenizer-upload-submit');
    const originalButtonText = submitButton.textContent;
    submitButton.innerHTML = '<span class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></span> Uploading...';

    fetch('/tab-third-party-apis-upload-tokenizer', {
        method: 'POST',
        body: formData
    })
    .then(response => {
        if (!response.ok) {
            return response.json().then(data => {
                throw new Error(data.detail || "Failed to upload tokenizer");
            });
        }
        return response.json();
    })
    .then(() => {
        const modal = bootstrap.Modal.getInstance(modalElement);
        if (modal) {
            modal.hide();
        }
        showSuccessToast("Tokenizer uploaded successfully");
        loadTokenizers();
        loadTokenizersForDropdown();
    })
    .catch(error => {
        console.error("Error uploading tokenizer:", error);
        errorElement.textContent = error.message || "Failed to upload tokenizer";
        errorElement.classList.remove('d-none');
    })
    .finally(() => {
        allButtons.forEach(btn => btn.disabled = false);
        allInputs.forEach(input => input.disabled = false);
        if (modalOverlay && modalOverlay.parentNode) {
            modalOverlay.parentNode.removeChild(modalOverlay);
        }
        submitButton.disabled = false;
        submitButton.textContent = originalButtonText;
    });
}

function deleteTokenizer(tokenizerId) {
    if (confirm("Are you sure you want to delete this tokenizer?")) {
        fetch("/tab-third-party-apis-delete-tokenizer", {
            method: "POST",
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ tokenizer_id: tokenizerId })
        })
        .then(response => {
            if (!response.ok) {
                return response.json().then(data => {
                    throw new Error(data.detail || "Failed to delete tokenizer");
                });
            }
            return response.json();
        })
        .then(() => {
            showSuccessToast("Tokenizer deleted successfully");
            loadTokenizers();
            loadTokenizersForDropdown();
        })
        .catch(error => {
            console.error("Error deleting tokenizer:", error);
            general_error(error);
        });
    }
}