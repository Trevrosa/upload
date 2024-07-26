"use strict";

/**
 * @type {HTMLButtonElement}
 */
const button = document.getElementById("submit");

/**
 * @type {HTMLParagraphElement}
 */
const defaultLogger = document.getElementById("defaultStatus");

var uploading = false;

/**
 * @param {File} _file 
 * @param {string} token
 * @param {HTMLDivElement} logger
 * @param {string?} _name
 */
function upload(_file, token, logger, _name = null) {
    let name = _file.name;
    if (_name !== null) {
        name = _name;
    }

    const file = new File([_file], name, { type: _file.type });

    const maxSize = 5_000_000;

    // if file size >5 MB, split up requests
    if (file.size > maxSize) {
        // can use because .dev websites require HSTS (https always)
        const id = window.crypto.randomUUID();
        const totalChunks = Math.ceil(file.size / maxSize);

        let num = 0;

        let erroredNum = 0;
        let finishedNum = 0;

        let collapsed = false;
        let done = false;
        let doneProcessing = false;

        const mainLogger = logger.getElementsByClassName("main")[0];
        const oldStatus = _file.name == file.name ? `uploading "${file.name}"` : `uploading "${_file.name}" as "${file.name}"`;
        
        mainLogger.innerHTML = oldStatus;
        mainLogger.title = "click me";
        mainLogger.style.cursor = "pointer";
        mainLogger.style.textDecoration = "underline";

        function toggleCollapse() {
            for (var i = 0; i < mainLogger.parentNode.children.length; i++) {
                const child = mainLogger.parentNode.children[i];
                if (child.classList.contains("main")) {
                    continue;
                }
                if (child.style.display == "block" | child.style.display == "") {
                    child.style.display = "none";
                } else {
                    child.style.display = "block";
                }
            }
            collapsed = !collapsed;

            if (collapsed && !doneProcessing) {
                mainLogger.innerHTML += " (collapsed)";
            } else {
                mainLogger.innerHTML = mainLogger.innerHTML.replace(" (collapsed)", "");
            }
        };
        
        mainLogger.onclick = (ev) => {
            if (ev.target.nodeName == "BUTTON") {
                return;
            }

            toggleCollapse();
        };

        setInterval(() => {
            if (done) {
                return;
            }
            const msg = collapsed ? " (collapsed)" : "";
            mainLogger.innerHTML = `${oldStatus}: ${finishedNum}/${totalChunks} chunks done, ${erroredNum} chunks errored` + msg;
        }, 500);

        setInterval(() => {
            if (finishedNum != totalChunks || done) {
                return;
            }

            done = true;
            uploading = false;

            const msg = collapsed ? " (collapsed)" : "";
            mainLogger.innerHTML = `${oldStatus}: almost done..` + msg;

            function finish() {
                const request = new XMLHttpRequest();

                request.onload = () => {
                    const response = request.response;
                    mainLogger.style.textDecoration = null;

                    doneProcessing = true;

                    if (request.status == 201) {
                        if (!collapsed) { toggleCollapse(); }

                        mainLogger.onclick = null;
                        mainLogger.style.cursor = null;
                        mainLogger.innerHTML = `uploaded! see <a href="${response}">${response}</a>`;
                    } else {
                        mainLogger.innerHTML = `${oldStatus}\n\n<div style="color: #cc0000; display: inline-block;">${response}</div>`;
                        
                        const button = document.createElement("button");
                        button.style.marginLeft = "5px";
                        button.innerText = "retry?";
                        button.onclick = finish;

                        mainLogger.appendChild(button);
                    }
                };

                request.onerror = () => {
                    mainLogger.style.textDecoration = null;
                    mainLogger.innerHTML = `${oldStatus}\n\n<div style="color: #cc0000; display: inline-block;">request error: ${request.status}</div>`;
                    
                    const button = document.createElement("button");
                    button.style.marginLeft = "5px";
                    button.innerText = "retry?";
                    button.onclick = finish;

                    mainLogger.appendChild(button);
                };
                
                request.open("PUT", `/upload/upload/done/${id}/${file.name}/${num}`);
                request.setRequestHeader("token", token);
                request.send();
            }

            finish();
        }, 500);
        
        for (let start = 0; start < file.size; start += maxSize) {
            num += 1;
            
            const chunk = file.slice(start, start + maxSize);
            const form = new FormData();
            form.set("file", chunk);
            
            const chunkLogger = document.createElement("p"); 
            logger.appendChild(chunkLogger);
            
            function uploadChunk() {
                chunkLogger.innerText = `chunk #${num} initializing..`;
    
                const request = new XMLHttpRequest();
    
                const cnum = num;
                request.upload.onprogress = (ev) => {
                    const percent = Math.round((ev.loaded / ev.total) * 100);
                    chunkLogger.innerText = `chunk #${cnum}: ${ev.loaded} out of ${ev.total} bytes (${percent}%)`;
                    
                    if (ev.loaded == ev.total) {
                        chunkLogger.innerText += `, wait..`;
                    }
                };
    
                request.onload = () => {
                    if (request.status == 201) {
                        finishedNum += 1;
                        chunkLogger.innerText += " done!";
                    } else {
                        uploading = false;
                        erroredNum += 1;
    
                        chunkLogger.innerHTML = `chunk #${cnum}: <div style="color: #cc0000; display: inline-block;">${request.response}</div>`;
                        
                        const button = document.createElement("button");
                        button.innerText = "retry?";
                        button.style.marginLeft = "5px";
                        button.onclick = uploadChunk;

                        chunkLogger.appendChild(button);
                    }
                };
    
                request.upload.onerror = () => {
                    erroredNum += 1;
                    chunkLogger.innerHTML = `chunk #${cnum}: <div style="color: #cc0000; display: inline-block;">request error ${request.status}</div>`;
                    
                    const button = document.createElement("button");
                    button.innerText = "retry?";
                    button.style.marginLeft = "5px";
                    button.onclick = uploadChunk;

                    chunkLogger.appendChild(button);
                };
    
                request.open("PUT", `/upload/upload/multi/${id}/${num}`);
                request.setRequestHeader("token", token);
                request.send(form);
            }

            uploadChunk();
        }
    } else {
        const form = new FormData();
        form.set("file", file);

        const defaultLogger = logger.firstChild;
        
        const oldStatus = _file.name == file.name ? `uploading "${file.name}"` : `uploading "${_file.name}" as "${file.name}"`;
        defaultLogger.innerText = oldStatus;

        const request = new XMLHttpRequest();

        request.onload = () => {
            uploading = false;
            const response = request.response;

            if (request.status == 201) {
                defaultLogger.innerHTML = `uploaded! see <a href="${response}">${response}</a>`;
            } else if (request.status == 502) {
                defaultLogger.innerHTML = "server offline; try again later";
            } else if (request.status == 520) {
                defaultLogger.innerHTML = "server error; try again";
            } else {
                defaultLogger.innerHTML += `\n\n<div style="color: #cc0000;">${response}</div>`;
            }
        };

        request.upload.onerror = () => {
            uploading = false;
            defaultLogger.innerHTML += `\n\n<div style="color: #cc0000;">request error ${request.status}</div>`;
        };

        request.upload.onprogress = (ev) => {
            const percent = Math.round((ev.loaded / ev.total) * 100);
            defaultLogger.innerHTML = `${oldStatus}: uploaded ${ev.loaded} out of ${ev.total} bytes (${percent}%)`;

            if (ev.loaded == ev.total) {
                defaultLogger.innerHTML += ", wait..";
            }
        };

        request.open("PUT", "/upload/upload/");
        request.setRequestHeader("token", token);
        request.send(form);
    }
}

document.getElementById("file").onchange = () => {
    /**
     * @type {File[]}
     */
    const files = document.getElementById("file").files;
    const names = document.getElementsByClassName("name");
    
    if (files.length == 1) {
        for (const name of names) {
            name.hidden = false;
            if (name.id == "name") {
                name.value = files[0].name;
            }
        }
    } else {
        for (const name of names) {
            name.hidden = true;
        }
    }
};

button.onclick = () => {
    if (uploading) {
        return;
    }

    /**
     * @type {File[]}
     */
    const files = document.getElementById("file").files;

    if (files.length == 0) {
        defaultLogger.firstChild.innerText = "u dont put file";
        return;
    }

    /**
     * @type {string}
     */
    const token = document.getElementById("token").value;

    if (token === "") {
        defaultLogger.firstChild.innerText = "u dont put password";
        return;
    }

    const statuses = document.getElementById("statuses");

    // clear statuses
    while (statuses.children.length != 1) {
        if (statuses.lastChild.id != "defaultStatus") {
            statuses.lastChild.remove();
        }
    }
    
    defaultLogger.innerHTML = '<p class="main" style="white-space: pre; font-weight: bold;"></p>';

    uploading = true;

    if (files.length == 1) {
        upload(files[0], token, defaultLogger, document.getElementById("name").value);
    } else {
        let first = true;

        for (const file of files) {
            if (first) {
                first = false;
                upload(file, token, defaultLogger);
                continue;
            }

            const logger = document.createElement("div");
            const status = document.createElement("p");
            status.style = "white-space: pre; font-weight: bold;";
            status.className = "main";

            statuses.appendChild(logger);
            logger.appendChild(status);

            upload(file, token, logger);
        }
    }
};
