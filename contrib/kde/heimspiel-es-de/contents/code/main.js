// ES-DE on KDE Plasma (KWin 6), for heimspiel (https://github.com/aronjanosch/heimspiel).
//
// Once every window that opened while ES-DE was running has closed again, ES-DE is made
// fullscreen and active, so the controller works in it right away. Games are left alone.
// Untested so far.

function isEsde(window) {
    const name = String(window.resourceClass).toLowerCase();
    return name === "es-de" || name === "org.es_de.frontend";
}

function esde() {
    return workspace.windowList().find(isEsde);
}

// Windows that opened while ES-DE was running: games, their launchers, heimspiel's loading window.
const launched = new Set();

workspace.windowAdded.connect(function (window) {
    if (window.normalWindow && !isEsde(window) && esde()) {
        launched.add(window.internalId.toString());
    }
});

workspace.windowRemoved.connect(function (window) {
    if (!launched.delete(window.internalId.toString()) || launched.size > 0) {
        return;
    }
    const frontend = esde();
    if (frontend) {
        frontend.fullScreen = true;
        workspace.activeWindow = frontend;
    }
});
