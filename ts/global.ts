const fadeInPage = () => {
    let fader = document.getElementById("fader")!;
    fader.classList.remove("init-in");
    fader.classList.remove("fade-in");
    fader.classList.add("fade-out");
}

document.addEventListener("DOMContentLoaded", () => {
    if (!window.AnimationEvent) { return; }

    setTimeout(fadeInPage, 10);

    const anchors = document.getElementsByTagName("a");
    for (let i=0; i < anchors.length; i++) {
        if (anchors[i].hostname !== window.location.hostname || anchors[i].pathname === window.location.pathname || anchors[i].target === "_blank")
            continue;

        anchors[i].addEventListener("click", (ev) => {
            const fader = document.getElementById("fader")!;
            const anchor = (ev.currentTarget! as HTMLAnchorElement)!;

            const listener = () => {
                window.location.href = anchor.href;
                fader?.removeEventListener("animationend", listener);
            }
            fader.addEventListener("animationend", listener);

            ev.preventDefault();

            fader.classList.remove("fade-out");
            fader.classList.add("fade-in");
        });
    }
});

// Make sure `fade-in` doesn't persist
window.addEventListener("pageshow", (ev) => {
    if (!ev.persisted) {
        return;
    }
    let fader = document.getElementById("fader")!;
    fader.classList.remove("fade-in");
    fader.classList.add("fade-out");
});

// Add id's to every header for Markdown sublinks
document.addEventListener("DOMContentLoaded", () => {
    let headings = document.querySelectorAll("h1, h2, h3, h4, h5");

    for (let i=0; i < headings.length; i++) {
        const heading = headings[i] as HTMLHeadingElement;
        heading.id = heading.innerText.toLowerCase().replace(' ', '_');
    }
});