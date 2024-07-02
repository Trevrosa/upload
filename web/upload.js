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
 * @param {File} file 
 * @param {Headers} auth 
 * @param {HTMLParagraphElement} logger
 */
async function upload(file, auth, logger) {
    // if file size >90 MB, split up requests
    if (file.size > 90_000_000) {
        let id = self.crypto.randomUUID();
        let num = 0;

        for (let start = 0; start < file.size; start += 90_000_000) {
            num += 1;
            
            let chunk = file.slice(start, start + 90_000_000);
            let form = new FormData();
            form.set("file", chunk);

            logger.innerText = `uploading "${file.name}" (big), wait`;
            await fetch(`/upload/upload/multi/${id}/${num}`, {
                method: "PUT",
                headers: auth,
                body: form
            }).then(async (resp) => {
                if (resp.status === 201) {
                    logger.innerText += "..";
                    if (chunk.size < 90_000_000) {
                        logger.innerText += " almost done..";
                        fetch(`/upload/upload/done/${id}/${file.name}`, {
                            method: "PUT",
                            headers: auth,
                        }).then(async (resp) => {
                            let response = await resp.text();
                            uploading = false;

                            if (resp.status === 201) {
                                logger.innerHTML = `uploaded! see <a href="${response}">${response}</a>`
                            } else {
                                logger.innerHTML += `\n\n<div style="color: #cc0000;">${response}</div>`;
                            }
                        }).catch((err) => {
                            uploading = false;
                            logger.innerHTML += `\n\n<div style="color: #cc0000;">${err}</div>`;
                        });
                    }
                } else {
                    let response = await resp.text();
                    uploading = false;
                    logger.innerHTML += `\n\n<div style="color: #cc0000;">${response}</div>`;
                }
            }).catch((err) => {
                uploading = false;
                logger.innerHTML += `\n\n<div style="color: #cc0000;">${err}</div>`;
            });
        }
    } else {
        let form = new FormData();
        form.set("file", file);

        logger.innerText = `uploading "${file.name}", wait`;
        fetch("/upload/upload/", {
            method: "PUT",
            headers: auth,
            body: form
        }).then(async (resp) => {
            let response = await resp.text();
            uploading = false;

            if (resp.status === 201) {
                logger.innerHTML = `uploaded! see <a href="${response}">${response}</a>`
            }
            else if (resp.status === 502) {
                logger.innerText = "server offline; try again later";
            } else if (resp.status === 520) {
                logger.innerText = "server error; try again";
            } else {
                logger.innerHTML += `\n\n<div style="color: #cc0000;">${response}</div>`;
            }
        }).catch((err) => {
            uploading = false;
            logger.innerHTML += `\n\n<div style="color: #cc0000;">${err}</div>`;
        });
    }
}

button.addEventListener("click", async () => {
    if (uploading) {
        return;
    }

    /**
     * @type {File[]}
     */
    let files = document.getElementById("file").files;

    if (files.length == 0) {
        defaultLogger.innerText = "u dont put file";
        return;
    }

    /**
     * @type {string}
     */
    let token = document.getElementById("token").value;

    if (token === "") {
        defaultLogger.innerText = "u dont put password";
        return;
    }

    let auth = new Headers();
    auth.set("token", token);

    let statuses = document.getElementById("statuses");

    // clear statuses
    while (statuses.children.length != 1) {
        if (statuses.lastChild.id != "defaultStatus") {
            statuses.lastChild.remove();
        }
    }

    uploading = true;

    if (files.length == 0) {
        await upload(files[0], auth, defaultLogger);
    } else {
        let first = true;

        for (const file of files) {
            if (first) {
                first = false;
                await upload(file, auth, defaultLogger);
                continue;
            }

            let logger = document.createElement("p");
            logger.style = "white-space: pre;";
            statuses.appendChild(logger);

            await upload(file, auth, logger);
        }
    }
});
