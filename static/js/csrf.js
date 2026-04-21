"use strict";

function getCsrfToken() {
    const prefix = "CSRF-TOKEN=";
    for (const raw of document.cookie.split("; ")) {
        if (raw.startsWith(prefix)) {
            return decodeURIComponent(raw.slice(prefix.length));
        }
    }
    return "";
}

document.addEventListener("htmx:configRequest", (evt) => {
    const token = getCsrfToken();
    if (token) evt.detail.headers["X-CSRF-Token"] = token;
});