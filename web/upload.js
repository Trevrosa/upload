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
 * @param {String} token
 * @param {HTMLDivElement} logger
 * @param {String?} _name
 */
async function upload(_file, token, logger, _name = null) {
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
        let finishedNum = 0;
        /**
         * @type {Set<Number>}
         */
        const erroredChunks = new Set();
        /**
         * @type {Set<Number>}
         */
        const retryingChunks = new Set();
        let sendingChunks = 0;

        let collapsed = false;
        let done = false;
        let doneProcessing = false;

        const mainLogger = logger.getElementsByClassName("main")[0];
        const oldStatus = _file.name == file.name ? `uploading "${file.name}"` : `uploading "${_file.name}" as "${file.name}"`;
        
        mainLogger.innerHTML = oldStatus;
        mainLogger.title = "click me";
        mainLogger.style.cursor = "pointer";
        mainLogger.style.textDecoration = "underline";
        mainLogger.style.userSelect = "none";

        function toggleCollapse() {
            for (var i = 0; i < mainLogger.parentNode.children.length; i++) {
                const child = mainLogger.parentNode.children[i];
                if (child.classList.contains("main")) {
                    continue;
                }
                // if shown, hide
                if (!collapsed) {
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
        
        // collapse
        mainLogger.onclick = (ev) => {
            if (ev.target.nodeName == "BUTTON") {
                return;
            }

            toggleCollapse();
        };

        // chunk status
        setInterval(() => {
            if (done) {
                return;
            }

            const collapsedMsg = collapsed ? " (collapsed)" : "";
            const retryingMsg = retryingChunks.size > 0 ? `, ${retryingChunks.size} chunks retrying` : "";
            const percent = Math.round((finishedNum / totalChunks) * 100);

            mainLogger.innerHTML = `${oldStatus}: ${finishedNum}/${totalChunks} chunks done (${percent}%), ${erroredChunks.size} chunks errored` + retryingMsg + collapsedMsg;
            
            // retry all
            if (erroredChunks.size > 0) {
                const button = document.createElement("button");
                button.style.marginLeft = "5px";
                button.innerText = "retry all?";
                button.id = "retryAll";

                button.onclick = async () => {
                    for (const erroredChunk of erroredChunks) {
                        const chunkLogger = logger.getElementsByClassName(erroredChunk)[0];
                        await uploadChunk(chunkLogger, true, erroredChunk);
                    }
                };

                mainLogger.appendChild(button);
            }
        }, 500);

        let firstFinish = true;

        // finish multi
        setInterval(() => {
            if (finishedNum != totalChunks || done) {
                return;
            }

            done = true;
            uploading = false;

            if (firstFinish) {
                firstFinish = false;
                if (!collapsed) { toggleCollapse(); }
            }

            const collapseMsg = collapsed ? " (collapsed)" : "";
            mainLogger.innerHTML = `${oldStatus}: almost done..` + collapseMsg;

            function finish() {
                const finishing = new EventSource(`/upload/upload/done/${id}/${file.name}/${totalChunks}`);

                finishing.onmessage = (msg) => {
                    if (msg.lastEventId != "progress") {
                        finishing.close();
                    }

                    mainLogger.style.textDecoration = null;

                    doneProcessing = true;

                    if (msg.lastEventId == "done") {
                        if (!collapsed) { toggleCollapse(); }

                        mainLogger.onclick = null;
                        mainLogger.style.cursor = null;
                        mainLogger.style.userSelect = "none";

                        mainLogger.innerHTML = `uploaded! see <a href="${msg.data}">${msg.data}</a>`;
                    } else if (msg.lastEventId == "progress") {
                        mainLogger.innerHTML = `${oldStatus}: almost done.. (${msg.data}/${totalChunks})` + collapseMsg;
                    } else {
                        mainLogger.style.userSelect = "";
                        mainLogger.innerHTML = `${oldStatus}\n\n<div style="color: #cc0000; display: inline-block;">${msg.data}</div>`;
                    }

                    if (msg.lastEventId != "done" && msg.lastEventId != "progress" && msg.lastEventId != "duplicate") {
                        const button = document.createElement("button");
                        button.style.marginLeft = "5px";
                        button.innerText = "retry?";
                        button.onclick = finish;

                        mainLogger.appendChild(button);
                    }
                };

                finishing.onerror = (err) => {
                    mainLogger.style.textDecoration = null;
                    mainLogger.innerHTML = `${oldStatus}\n\n<div style="color: #cc0000; display: inline-block;">EventSource failed, check console</div>`;
                    
                    console.error(err);

                    const button = document.createElement("button");
                    button.style.marginLeft = "5px";
                    button.innerText = "retry?";
                    button.onclick = finish;

                    mainLogger.appendChild(button);

                    // stop reconnecting
                    finishing.close();
                };
            }

            finish();
        }, 500);

        /**
         * upload a chunk
         * @param {HTMLParagraphElement} chunkLogger
         * @param {Boolean} retry  
         * @param {Number?} cnum
         */
        async function uploadChunk(chunkLogger, retry = false, _cnum = null) {
            chunkLogger.innerText = `chunk #${num} initializing..`;

            sendingChunks += 1;

            // only allow 10 chunks uploading at once to avoid timeout
            if (sendingChunks > 10) {
                while (true) {
                    await new Promise(r => setTimeout(r, 2000));
                    if (sendingChunks <= 10) {
                        break;
                    }
                }
            }

            const request = new XMLHttpRequest();

            let cnum;
            if (retry) {
                cnum = _cnum;
            } else {
                cnum = num;
            }

            if (retryingChunks.has(cnum)) {
                return;
            }

            if (retry) {
                retryingChunks.add(cnum);
            }

            const chunk = file.slice((cnum-1) * maxSize, cnum * maxSize);
            const form = new FormData();
            const hash = XXH.h32(await chunk.arrayBuffer(), 0);

            form.set("file", chunk);
            form.set("hash", hash);

            request.upload.onprogress = (ev) => {
                const percent = Math.round((ev.loaded / ev.total) * 100);
                chunkLogger.innerText = `chunk #${cnum}: ${ev.loaded} out of ${ev.total} bytes (${percent}%)`;
                
                if (ev.loaded == ev.total) {
                    chunkLogger.innerText += `, wait..`;
                }
            };

            request.onload = () => {
                retryingChunks.delete(cnum);
                sendingChunks -= 1;
                
                if (request.status == 201) {
                    erroredChunks.delete(cnum);

                    finishedNum += 1;
                    chunkLogger.innerText += " done!";
                } else if (request.status == 409) { // conflict, means already exist
                    erroredChunks.delete(cnum);
                    
                    finishedNum += 1;
                    chunkLogger.innerText += " done! (already uploaded)";
                } else {
                    uploading = false;
                    erroredChunks.add(cnum);

                    chunkLogger.innerHTML = `chunk #${cnum}: <div style="color: #cc0000; display: inline-block;">${request.response}</div>`;
                    
                    const button = document.createElement("button");
                    button.innerText = "retry?";
                    button.style.marginLeft = "5px";
                    button.onclick = async () => {
                        await uploadChunk(chunkLogger, true, cnum);
                    };

                    chunkLogger.appendChild(button);
                }
            };

            request.upload.onerror = () => {
                erroredChunks.add(cnum);
                chunkLogger.innerHTML = `chunk #${cnum}: <div style="color: #cc0000; display: inline-block;">request error ${request.status}</div>`;
                
                const button = document.createElement("button");
                button.innerText = "retry?";
                button.style.marginLeft = "5px";
                button.onclick = async () => {
                    await uploadChunk(chunkLogger, true, cnum);
                };

                chunkLogger.appendChild(button);
            };

            request.open("POST", `/upload/upload/multi/${id}/${cnum}`);
            request.setRequestHeader("token", token);
            request.send(form);
        }
        
        // upload chunks
        for (let start = 0; start < file.size; start += maxSize) {
            num += 1;
            
            const chunkLogger = document.createElement("p");
            chunkLogger.className = num;
            if (collapsed) {
                chunkLogger.style.display = "none";
            }
            logger.appendChild(chunkLogger);

            await uploadChunk(chunkLogger);
        }
    } else {
        const form = new FormData();
        form.set("file", file);

        const defaultLogger = logger.firstChild;
        
        const oldStatus = _file.name == file.name ? `uploading "${file.name}"` : `uploading "${_file.name}" as "${file.name}"`;
        defaultLogger.innerText = oldStatus;

        function uploadSmall() {
            const request = new XMLHttpRequest();

            request.onload = () => {
                uploading = false;
                const response = request.response;

                if (request.status == 201) {
                    defaultLogger.innerHTML = `uploaded! see <a href="${response}">${response}</a>`;
                } else if (request.status == 502) {
                    defaultLogger.innerHTML += "\n\n<div style='color: #cc0000; display: inline-block;'>server offline</div>";
                } else if (request.status == 520) {
                    defaultLogger.innerHTML += "\n\n<div style='color: #cc0000; display: inline-block;'>server error</div>";
                } else {
                    defaultLogger.innerHTML += `\n\n<div style="color: #cc0000; display: inline-block;">${response}</div>`;
                }

                // 409 = conflict (duplicate file), 403 = forbidden
                if (request.status != 201 && request.status != 409 && request.status != 403) {
                    const button = document.createElement("button");
                    button.style.marginLeft = "5px";
                    button.innerText = "retry?";
                    button.onclick = uploadSmall;

                    defaultLogger.appendChild(button);
                }
            };

            request.upload.onerror = () => {
                uploading = false;
                defaultLogger.innerHTML += `\n\n<div style="color: #cc0000; display: inline-block;">request error ${request.status}</div>`;

                const button = document.createElement("button");
                button.style.marginLeft = "5px";
                button.innerText = "retry?";
                button.onclick = uploadSmall;

                defaultLogger.appendChild(button);
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

        uploadSmall();
    }
}

// allow rename single file
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

document.getElementById("token").onload = () => {
    const token = localStorage.getItem("token");
    if (token) {
        document.getElementById("token").value = token;
    }
};

document.getElementById("token").onchange = () => {
    localStorage.setItem("token", document.getElementById("token").value);
};

button.onclick = async () => {
    if (uploading) {
        return;
    }

    /**
     * @type {File[]}
     */
    const files = document.getElementById("file").files;

    if (files.length == 0) {
        defaultLogger.children[0].innerHTML = "u dont put file";
        return;
    }

    /**
     * @type {String}
     */
    const token = document.getElementById("token").value;

    if (token === "") {
        defaultLogger.children[0].innerHTML = "u dont put password";
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

            await upload(file, token, logger);
        }
    }
};
