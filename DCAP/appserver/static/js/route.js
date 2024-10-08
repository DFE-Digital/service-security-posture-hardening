let change = (mutationList, observer) => {

    console.log("Routing table mutation detected");

    for (var mutationListIter = 0; mutationListIter < mutationList.length; mutationListIter++) {

        let tds = mutationList[mutationListIter].target.getElementsByTagName("TD");

        for (var tdsIter = 0; tdsIter < tds.length; tdsIter++) {

            if (tds[tdsIter].classList.contains("string")) {

                console.log("Looking for URL");

                let new_location = tds[tdsIter].innerHTML;

                console.log(`Found URL: ${new_location}`);

                window.location.replace(new_location);

            }
        }
    }
};

console.log("Starting Routing observations");

const observer = new MutationObserver(change);
const config = { attributes: true, childList: true, subtree: true };

let targetNode = document.getElementById("routing_table");

observer.observe(targetNode, config);


console.log("Routing observations complete");
