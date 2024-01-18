const fs = require('fs').promises;


const main = async () => {
    // load all folders in the cache directory
    const folders = await fs.readdir('./cache');
    // iterate over each folder
    //
    for (const folder of folders) {
        // read all the files in the folder
        if (folder == ".DS_Store") {
            continue;
        }
        const files = await fs.readdir(`./cache/${folder}`);
        // iterate over each file

        for (const file of files) {
            // read out the file
            const data = await fs.readFile(`./cache/${folder}/${file}`, 'utf-8');
            // parse the file
            const json = JSON.parse(data);
            if (json.length === 0) {
                // if the file is empty, delete it
                await fs.unlink(`./cache/${folder}/${file}`);
                console.log(`Deleted ${folder}/${file}`);
            }
        }
    }
}
main();
