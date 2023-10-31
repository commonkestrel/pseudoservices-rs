const fadeInPage = () => {
    let wiper = document.getElementById("wiper")!;
    wiper.classList.remove("init-in");
    wiper.classList.remove("fade-in");
    wiper.classList.add("fade-out");
}

document.addEventListener("DOMContentLoaded", () => {
    if (!window.AnimationEvent) { return; }

    setTimeout(fadeInPage, 10);

    const anchors = document.getElementsByTagName("a");
    for (let i=0; i < anchors.length; i++) {
        if (anchors[i].hostname !== window.location.hostname || anchors[i].pathname === window.location.pathname || anchors[i].target === "_blank")
            continue;

        anchors[i].addEventListener("click", (ev) => {
            const wiper = document.getElementById("wiper")!;
            const anchor = (ev.currentTarget! as HTMLAnchorElement)!;

            const listener = () => {
                window.location.href = anchor.href;
                wiper?.removeEventListener("animationend", listener);
            }
            wiper.addEventListener("animationend", listener);

            ev.preventDefault();

            wiper.classList.remove("fade-out");
            wiper.classList.add("fade-in");
        });
    }
});

// Make sure `fade-in` doesn't persist
window.addEventListener("pageshow", (ev) => {
    if (!ev.persisted) {
        return;
    }
    let wiper = document.getElementById("wiper")!;
    wiper.classList.remove("fade-in");
    wiper.classList.add("fade-out");
});

// Add id's to every header for Markdown sublinks
document.addEventListener("DOMContentLoaded", () => {
    let headings = document.querySelectorAll("h1, h2, h3, h4, h5");

    for (let i=0; i < headings.length; i++) {
        const heading = headings[i] as HTMLHeadingElement;
        if (!heading.id) {
            heading.id = heading.innerText.toLowerCase().replace(/ /g, '_');
        }
    }
});