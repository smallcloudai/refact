import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

const site = 'https://docs.refact.ai/';

// https://astro.build/config
export default defineConfig({
  integrations: [
    starlight({
      title: 'Refact Documentation',
      components: {
        Search: './src/components/Search.astro',
        Head: './src/components/Head.astro'
      },
      logo: {
        light: '/src/assets/logo-light.svg',
        dark: '/src/assets/logo-dark.svg',
        replacesTitle: true,
      },
      social: {
        github: 'https://github.com/smallcloudai',
        discord: 'https://smallcloud.ai/discord'
      },
      head: [
        {
          tag: 'meta',
          attrs: { property: 'og:image', content: site + 'og.jpg' }
        },
        {
          tag: 'meta',
          attrs: { property: 'twitter:image', content: site + 'og.jpg' }
        },
        {
          tag: 'script',
          attrs: {
            async: true,
            src: 'https://www.googletagmanager.com/gtag/js?id=G-76LB6JQLMK',
          },
        },
        {
          tag: 'script',
          content: `
						window.dataLayer = window.dataLayer || [];
						function gtag() {
							dataLayer.push(arguments);
						}
						gtag('js', new Date());

						gtag('config', 'G-76LB6JQLMK');
					`,
        }
      ],
      sidebar: [
        {
          label: 'Introduction',
          collapsed: true,
          items: [
            { 
              label: 'Quickstart', 
              link: '/introduction/quickstart/',
              attrs: {
                'aria-label': 'Get started with Refact'
              }
            },
            {
              label: 'Installation',
              collapsed: true,
              items: [
                { 
                  label: 'Installation Hub', 
                  link: '/installation/installation-hub/',
                  attrs: {
                    'aria-label': 'Browse Installation Options'
                  }
                },
                { 
                  label: 'VS Code', 
                  link: '/installation/vs-code/',
                  attrs: {
                    'aria-label': 'Install Refact for VS Code'
                  }
                },
                { 
                  label: 'JetBrains IDEs', 
                  link: '/installation/jetbrains/',
                  attrs: {
                    'aria-label': 'Install Refact for JetBrains IDEs'
                  }
                },
              ] 
            },
            {
              label: 'Features',
              collapsed: true,
              items: [
                { 
                  label: 'AI Chat', 
                  link: '/features/ai-chat/',
                  attrs: {
                    'aria-label': 'Learn about AI Chat Feature'
                  }
                },
                { 
                  label: 'AI Toolbox', 
                  link: '/features/ai-toolbox/',
                  attrs: {
                    'aria-label': 'Explore AI Toolbox Features'
                  }
                },
                { 
                  label: 'Code Completion', 
                  link: '/features/code-completion/',
                  attrs: {
                    'aria-label': 'Learn about Code Completion'
                  }
                },
                { 
                  label: 'Context', 
                  link: '/features/context/',
                  attrs: {
                    'aria-label': 'Understanding Context Features'
                  }
                },
                { 
                  label: 'Fine-tuning', 
                  link: '/features/finetuning/',
                  attrs: {
                    'aria-label': 'Learn about Model Fine-tuning'
                  }
                },
              ]
            },
          ],
        },
        {
          label: 'Autonomous Agent',
          collapsed: true,
          items: [
            { label: 'Getting Started', link: '/features/autonomous-agent/getting-started/' },
            { label: 'Overview', link: '/features/autonomous-agent/overview/' },
            { 
              label: 'Tools', 
              link: '/features/autonomous-agent/tools/',
              attrs: {
                'aria-label': 'Learn about Agent Tools'
              }
            },
            { 
              label: 'Rollback', 
              link: '/features/autonomous-agent/rollback/',
              attrs: {
                'aria-label': 'Learn about Agent Rollback Feature'
              }
            },
            { 
              label: 'Integrations', 
              collapsed: true,
              items: [
                { label: 'Overview', link: '/features/autonomous-agent/integrations/' },
                // Development Tools
    		{ label: 'Chrome', link: '/features/autonomous-agent/integrations/chrome/' },
                { label: 'Shell Commands', link: '/features/autonomous-agent/integrations/shell-commands/' },
                { label: 'Command Line Tool', link: '/features/autonomous-agent/integrations/command-line-tool/' },
                { label: 'Command Line Service', link: '/features/autonomous-agent/integrations/command-line-service/' },
                // Version Control
                { label: 'GitHub', link: '/features/autonomous-agent/integrations/github/' },
                { label: 'GitLab', link: '/features/autonomous-agent/integrations/gitlab/' },
                // Container Management
                { label: 'Docker', link: '/features/autonomous-agent/integrations/docker/' },
                // Databases
                { label: 'PostgreSQL', link: '/features/autonomous-agent/integrations/postgresql/' },
                { label: 'MySQL', link: '/features/autonomous-agent/integrations/mysql/' },
                // Debugging
                { label: 'PDB', link: '/features/autonomous-agent/integrations/pdb/' },
              ] 
            },
          ]
        },
        {
          label: 'Guides',
          collapsed: true,
          items: [
            { 
              label: 'Deployment',
              collapsed: true,
              items: [
                { 
                  label: 'Runpod Deployment', 
                  link: '/guides/deployment/runpod/',
                  attrs: {
                    'aria-label': 'Learn about Runpod Deployment'
                  }
                },
                { 
                  label: 'AWS Deployment', 
                  collapsed: true,
                  items: [
                    { 
                      label: 'Getting Started', 
                      link: '/guides/deployment/aws/getting-started/',
                      attrs: {
                        'aria-label': 'Getting Started with AWS Deployment'
                      }
                    },
                    { 
                      label: 'Launch from EC2', 
                      link: '/guides/deployment/aws/ec2/',
                      attrs: {
                        'aria-label': 'Launch Refact from EC2'
                      }
                    },
                    { 
                      label: 'Launch from Website', 
                      link: '/guides/deployment/aws/marketplace/',
                      attrs: {
                        'aria-label': 'Launch Refact from AWS Marketplace'
                      }
                    },
                    { 
                      label: 'Usage', 
                      link: '/guides/deployment/aws/usage/',
                      attrs: {
                        'aria-label': 'AWS Deployment Usage Guide'
                      }
                    },
                  ] 
                },
              ] 
            },
            {
              label: 'Plugins',
              collapsed: true,
              items: [
                { 
                  label: 'JetBrains IDEs', 
                  collapsed: true,
                  items: [
                    { 
                      label: 'Troubleshooting', 
                      link: '/guides/plugins/jetbrains/troubleshooting/',
                      attrs: {
                        'aria-label': 'JetBrains IDEs Troubleshooting Guide'
                      }
                    },
                  ]
                },
              ]
            },
            { 
              label: 'Authentication', 
              collapsed: true,
              items: [
                { 
                  label: 'Keycloak Integration', 
                  link: '/guides/authentication/keycloak/',
                  attrs: {
                    'aria-label': 'Learn about Keycloak Integration'
                  }
                },
              ]
            },
            { 
              label: 'Version-specific Usage',
              collapsed: true,
              items: [
                { 
                  label: 'Self-hosted Refact',
                  collapsed: true,
                  items: [
                    { 
                      label: 'Self-hosted Refact', 
                      link: '/guides/version-specific/self-hosted/',
                      attrs: {
                        'aria-label': 'Self-hosted Refact Guide'
                      }
                    }
                  ]
                },
                { 
                  label: 'Enterprise Refact', 
                  collapsed: true,
                  items: [
                    { 
                      label: 'Getting Started', 
                      link: '/guides/version-specific/enterprise/getting-started/',
                      attrs: {
                        'aria-label': 'Getting Started with Enterprise Refact'
                      }
                    },
                    { 
                      label: 'License', 
                      link: '/guides/version-specific/enterprise/license/',
                      attrs: {
                        'aria-label': 'Enterprise Refact License Information'
                      }
                    },
                    { 
                      label: 'Users', 
                      link: '/guides/version-specific/enterprise/users/',
                      attrs: {
                        'aria-label': 'Enterprise Refact User Management'
                      }
                    },
                    { 
                      label: 'Model Hosting', 
                      link: '/guides/version-specific/enterprise/model-hosting/',
                      attrs: {
                        'aria-label': 'Enterprise Refact Model Hosting Guide'
                      }
                    },
                    { 
                      label: 'Plugins', 
                      link: '/guides/version-specific/enterprise/plugins/',
                      attrs: {
                        'aria-label': 'Enterprise Refact Plugins Guide'
                      }
                    },
                  ] 
                },
                { 
                  label: 'Refact Teams', 
                  link: '/guides/version-specific/teams/',
                  attrs: {
                    'aria-label': 'Learn about Refact Teams'
                  }
                },
              ]
            },
            { 
              label: 'Reverse Proxy', 
              link: '/guides/reverse-proxy/',
              attrs: {
                'aria-label': 'Learn about Reverse Proxy Setup'
              }
            },
          ]
        },
        {
          label: 'Supported Models',
          link: '/supported-models/',
          attrs: {
            'aria-label': 'View Supported AI Models'
          }
        },
        {
          label: 'BYOK',
          link: '/byok/',
          attrs: {
            'aria-label': 'Learn about Bring Your Own Key (BYOK)'
          }
        },
        {
          label: 'FAQ',
          link: '/faq/',
          attrs: {
            'aria-label': 'Frequently Asked Questions'
          }
        },
        {
          label: 'Contributing',
          link: '/contributing/',
          attrs: {
            'aria-label': 'Learn how to contribute to Refact'
          }
        },
      ],
      customCss: [
        // Relative path to your custom CSS file
        './src/styles/custom.css',
      ],
      editLink: {
        baseUrl: 'https://github.com/smallcloudai/web_docs_refact_ai/edit/main/',
      },
      lastUpdated: true,
    }),
  ],
});
