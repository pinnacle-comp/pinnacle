import { defineConfig } from "vitepress"
import { tabsMarkdownPlugin } from "vitepress-plugin-tabs"

// https://vitepress.dev/reference/site-config
export default defineConfig({
    title: "Pinnacle Wiki",
    description: "Wiki for the Pinnacle Wayland compositor",
    themeConfig: {
        // https://vitepress.dev/reference/default-theme-config
        nav: [
            { text: "Home", link: "/" },
            { text: "Wiki", link: "/getting-started/introduction" }
        ],

        sidebar: [
            {
                text: "Getting Started",
                items: [
                    { text: "Introduction", link: "/getting-started/introduction" },
                    { text: "Installation", link: "/getting-started/installation" }
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
                    { text: "Integration with external applications", link: "/configuration/integration" },
                    { text: "Xwayland", link: "/configuration/xwayland" },
                ]
            },
        ],

        socialLinks: [
            { icon: "github", link: "https://github.com/pinnacle-comp/pinnacle" },
            { icon: "discord", link: "https://discord.gg/JhpKtU2aMA" },
            { icon: "matrix", link: "https://matrix.to/#/#pinnacle:matrix.org" },
        ]
    },
    markdown: {
        config(md) {
            md.use(tabsMarkdownPlugin)
        }
    }
})
