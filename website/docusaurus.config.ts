import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import {withProductSite} from '@beyond10x/docs-system/product-site';

const config: Config = {
  title: 'codespec',
  tagline: 'Code as typed data: read a language into cir/1, rewrite it, write another language from it, and prove the result behaves the same.',

  future: {
    v4: true,
  },

  url: 'https://beyond10x.github.io',
  baseUrl: '/codespec/',

  organizationName: 'beyond10x',
  projectName: 'codespec',
  deploymentBranch: 'gh-pages',
  trailingSlash: false,

  onBrokenLinks: 'throw',
  onBrokenAnchors: 'throw',

  markdown: {
    format: 'detect',
    hooks: {
      onBrokenMarkdownLinks: 'throw',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          routeBasePath: 'docs',
          editUrl: 'https://github.com/beyond10x/codespec/tree/main/website/',
        },
        blog: false,
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    navbar: {
      title: 'codespec',
      items: [
        {to: '/docs/', label: 'Documentation', position: 'left'},
        {to: '/docs/getting-started', label: 'Get started', position: 'left'},
        {to: '/docs/examples/greeter', label: 'Showcase', position: 'left'},
        {to: '/docs/reference/cir-1', label: 'cir/1', position: 'left'},
        {to: '/docs/reference/support', label: 'Support', position: 'left'},
        {to: '/docs/status', label: 'Status', position: 'left'},
        {href: 'https://github.com/beyond10x/codespec', label: 'GitHub', position: 'right'},
      ],
    },
    footer: {
      links: [
        {
          title: 'Documentation',
          items: [
            {label: 'Overview', to: '/docs/'},
            {label: 'Vision', to: '/docs/vision'},
            {label: 'Getting started', to: '/docs/getting-started'},
            {label: 'Showcase: greeter', to: '/docs/examples/greeter'},
            {label: 'Showcase: series', to: '/docs/examples/series'},
            {label: 'The cir/1 model', to: '/docs/concepts/cir-1'},
            {label: 'Readers, passes and writers', to: '/docs/concepts/pipeline'},
            {label: 'The harness', to: '/docs/concepts/harness'},
            {label: 'Add an adapter', to: '/docs/guides/add-an-adapter'},
            {label: 'Translate a Go package to Rust', to: '/docs/guides/translate-go-to-rust'},
            {label: 'codespec CLI', to: '/docs/reference/cli'},
            {label: 'cir/1 reference', to: '/docs/reference/cir-1'},
            {label: 'Language support', to: '/docs/reference/support'},
            {label: 'Status', to: '/docs/status'},
          ],
        },
        {
          title: 'Project',
          items: [
            {label: 'Source', href: 'https://github.com/beyond10x/codespec'},
            {label: 'Releases', href: 'https://github.com/beyond10x/codespec/releases'},
            {label: 'Changelog', href: 'https://github.com/beyond10x/codespec/blob/main/CHANGELOG.md'},
          ],
        },
      ],
      copyright: 'A beyond10x project. codespec · Apache-2.0.',
    },
  } satisfies Preset.ThemeConfig,
};

export default withProductSite(config, {landing: './product.json', mark: 'cs'});
