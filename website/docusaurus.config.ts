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
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themes: [
    [
      '@easyops-cn/docusaurus-search-local',
      {
        hashed: true,
        language: ['en'],
        indexDocs: true,
        indexBlog: false,
        indexPages: true,
        docsRouteBasePath: 'docs',
        explicitSearchResultPath: true,
        highlightSearchTermsOnTargetPage: false,
      },
    ],
  ],

  themeConfig: {
    docs: {
      sidebar: {autoCollapseCategories: true},
    },
    navbar: {
      title: 'codespec',
      items: [
        {to: '/docs/', label: 'Documentation', position: 'left'},
        {to: '/docs/getting-started', label: 'Getting started', position: 'left'},
        {to: '/docs/reference/cir-1', label: 'Reference', position: 'left', activeBasePath: '/docs/reference'},
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
            {label: 'Getting started', to: '/docs/getting-started'},
            {label: 'Showcase: greeter', to: '/docs/examples/greeter'},
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
