const CHARS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789)@=$#^*/\\]}~_:;`";
const FIRST_NAME = "Common"
const LAST_NAME = "Kestrel";

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

const randomize = () => {
    const wrapper = document.getElementById("string-wrapper")!;
    const wrapperStyle = window.getComputedStyle(wrapper);

    const fontHeight = parseFloat(wrapperStyle.fontSize);
    const fontWidth = fontHeight / 2;

    const box = wrapper.getBoundingClientRect();
    const stringHeight = box.height;
    const stringWidth = box.width;

    const horizontalChars = Math.ceil(stringWidth / fontWidth);
    const verticalChars = Math.ceil(stringHeight / fontHeight)
    const stringLength = Math.ceil(verticalChars * horizontalChars);
    
    const firstIndex = stringLength / 2 - horizontalChars / 2 - 5;
    const lastIndex = firstIndex + horizontalChars + 2; 
    
    const chars = randomString(stringLength);
    const hex = asciiToHex(chars);
    document.getElementById("string")!.innerHTML = chars.slice(0, firstIndex) + "<span class=\"redlight\">" + FIRST_NAME + "</span>" + chars.slice(firstIndex + FIRST_NAME.length, lastIndex) + "<span class=\"bluelight\">" + LAST_NAME + "</span>" + chars.slice(lastIndex + LAST_NAME.length);
    document.getElementById("hex")!.innerHTML = hex.slice(0, 3*firstIndex) + "<span class=\"redlight\">" + firstHex + " </span>" + hex.slice(3*firstIndex + firstHex.length, 3*lastIndex) + "<span class=\"bluelight\">" + lastHex + " </span>" + hex.slice(3*lastIndex + lastHex.length);
}

setInterval(randomize, 50);
