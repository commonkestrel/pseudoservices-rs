const CHARS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789)@=$#^*/\\]}~_:;`";
const FIRST_NAME = "Pseudo"
const LAST_NAME = "Services";

const randomChar = () => CHARS[Math.floor(Math.random() * (CHARS.length - 1))];
const randomString = (length: number) => Array.from(Array(length)).map(randomChar).join("");

const asciiToHex = (str: string) => {
    let ret = [];
    for (var n = 0, l = str.length; n < l; n ++) 
     {
		var hex = str.charCodeAt(n).toString(16).toUpperCase();
		ret.push(hex);
	 }
     return ret.join(' ');
}

const firstHex = asciiToHex(FIRST_NAME);
const lastHex = asciiToHex(LAST_NAME);

/**
 * The element passed must have two `<a>` tags present already.
 * @param el The element to populate
 */
const populateNodes = (el: HTMLElement) => {
    const children = el.childNodes;
    const textNode = document.createTextNode("");
    el.insertBefore(textNode, children[0]);
    el.insertBefore(textNode, children[2]);
    el.appendChild(textNode);
    return children;
}

const randomize = (strId: string, hexId?: string) => {
    const wrapper = document.getElementById("string-wrapper")!;
    const wrapperStyle = window.getComputedStyle(wrapper);

    const fontHeight = parseFloat(wrapperStyle.fontSize);
    const fontWidth = fontHeight * 3 / 5;

    const box = wrapper.getBoundingClientRect();
    const stringHeight = box.height;
    const stringWidth = box.width;

    const horizontalChars = Math.floor(stringWidth / fontWidth);
    const verticalChars = Math.ceil(stringHeight / fontHeight)
    const stringLength = Math.ceil(verticalChars * horizontalChars);
    
    const firstIndex = (Math.floor(verticalChars / 2) - 1) * horizontalChars + Math.ceil(horizontalChars / 2) - 5;
    const lastIndex = firstIndex + horizontalChars + 2; 

    const chars = randomString(stringLength);
    let strChildren = document.getElementById(strId)!.childNodes;
    if (strChildren.length == 0)
        populateNodes(document.getElementById(strId)!);
    strChildren[0].nodeValue = chars.slice(0, firstIndex);
    strChildren[2].nodeValue = chars.slice(firstIndex + FIRST_NAME.length, lastIndex);
    strChildren[4].nodeValue = chars.slice(lastIndex + LAST_NAME.length);

    if (hexId) {
        const hex = asciiToHex(chars);
        const hexChildren = document.getElementById(hexId)!.childNodes;
        if (hexChildren.length == 0)
            populateNodes(document.getElementById(hexId)!);
        hexChildren[0].nodeValue = hex.slice(0, 3*firstIndex);
        hexChildren[2].nodeValue = hex.slice(3*firstIndex + firstHex.length, 3*lastIndex);
        hexChildren[4].nodeValue = hex.slice(3*lastIndex + lastHex.length);
    }
}

let landscape = window.innerWidth > window.innerHeight;
window.addEventListener("resize", (ev) => {
    landscape = window.innerWidth > window.innerHeight;
});

setInterval(() => {
    if (landscape) 
        randomize("string", "hex");
    else
        randomize("string");
}, 70);
