import { register } from "../lib/html.js";

import Settings from "./settings.js";
import ServerList from "./server/list.js";
import ServerView from "./server/view.js";
import { App } from "./app.js";
import { Redirect } from "./router.js";

export { Settings, ServerList, ServerView, App };

register("Redirect", Redirect);
