import type { Theme } from "vitepress"
import { h } from "vue"
import DefaultTheme from "vitepress/theme"
import { enhanceAppWithTabs } from "vitepress-plugin-tabs/client"
import VersionSwitcher from "vitepress-versioning-plugin/src/components/VersionSwitcher.vue"
import "./tabs.css"
import "./custom.css"

import {
    NolebaseHighlightTargetedHeading,
} from "@nolebase/vitepress-plugin-highlight-targeted-heading/client"

import "@nolebase/vitepress-plugin-highlight-targeted-heading/client/style.css"

export default {
    extends: DefaultTheme,
    Layout() {
        return h(DefaultTheme.Layout, null, {
            "layout-top": () => [
                h(NolebaseHighlightTargetedHeading),
            ],
        })
    },
    enhanceApp({ app }) {
        enhanceAppWithTabs(app)
        app.component("VersionSwitcher", VersionSwitcher)
    }
} satisfies Theme
