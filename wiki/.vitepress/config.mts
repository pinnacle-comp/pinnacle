import { tabsMarkdownPlugin } from "vitepress-plugin-tabs"
import defineVersionedConfig from "vitepress-versioning-plugin"

// https://vitepress.dev/reference/site-config
export default defineVersionedConfig({
    title: "Pinnacle Wiki",
    description: "Wiki for the Pinnacle Wayland compositor",
    themeConfig: {
        // https://vitepress.dev/reference/default-theme-config
        nav: [
            { text: "Home", link: "/" },
            { text: "Wiki", link: "/getting-started/introduction" },
            { text: "Lua Reference", link: "https://pinnacle-comp.github.io/lua-reference/" },
            { text: "Rust Reference", link: "https://pinnacle-comp.github.io/rust-reference/main" },
            { component: "VersionSwitcher" },
        ],
        socialLinks: [
            { icon: "github", link: "https://github.com/pinnacle-comp/pinnacle" },
            { icon: "discord", link: "https://discord.gg/JhpKtU2aMA" },
            { icon: "matrix", link: "https://matrix.to/#/#pinnacle:matrix.org" },
        ],
        search: {
            provider: "local",
        },
        sidebar: {
            "/": [
                {
                    text: "Getting Started",
                    items: [
                        { text: "Introduction", link: "/getting-started/introduction" },
                        { text: "Installation", link: "/getting-started/installation" },
                        { text: "Running", link: "/getting-started/running" },
                    ]
                },
                {
                    text: "Configuration",
                    items: [
                        { text: "Creating a config", link: "/configuration/creating-a-config" },
                        { text: "Config basics", link: "/configuration/config-basics" },
                        { text: "Binds", link: "/configuration/binds" },
                        { text: "Input devices", link: "/configuration/input-devices" },
                        { text: "Tags", link: "/configuration/tags" },
                        { text: "Outputs", link: "/configuration/outputs" },
                        { text: "Windows", link: "/configuration/windows" },
                        { text: "Layouts", link: "/configuration/layout" },
                        { text: "Processes", link: "/configuration/processes" },
                        { text: "Snowcap", link: "/configuration/snowcap" },
                        { text: "Integration with external applications", link: "/configuration/integration" },
                        { text: "Xwayland", link: "/configuration/xwayland" },
                    ]
                },
            ],
        },
        versionSwitcher: false,
    },
    markdown: {
        config(md) {
            md.use(tabsMarkdownPlugin)
        }
    },
    versioning: {
        latestVersion: "main",
    },
    locales: {
        root: {
            label: "English",
            lang: "en",
        },
    },
    lastUpdated: true,
    vite: {
        ssr: {
            noExternal: [
                "@nolebase/vitepress-plugin-highlight-targeted-heading",
            ]
        }
    },
    base: "/pinnacle/"
}, __dirname)
