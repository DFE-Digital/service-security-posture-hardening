let change = (mutationList, observer) => {

    console.log("mutation detected");

    for (var mutationListIter = 0; mutationListIter < mutationList.length; mutationListIter++) {

        let tds = mutationList[mutationListIter].target.getElementsByTagName("TD");

        for (var tdsIter = 0; tdsIter < tds.length; tdsIter++) {

            if (tds[tdsIter].classList.contains("string") && !tds[tdsIter].classList.contains("ssphp_modified")) {

                console.log("mutatating string");

                tds[tdsIter].innerHTML = tds[tdsIter].innerHTML.replace(/~~(.*?)~~(.*?)~~/g, '<span class="$2">$1</span>');

                tds[tdsIter].innerHTML = tds[tdsIter].innerHTML.replace(/~!(.*?)~!(.*?)~!(.*?)~!/gs, '<$1>$2<$3>');

                tds[tdsIter].classList.add("ssphp_modified");

            }
        }

    }

};

console.log("Starting observations");

const observer = new MutationObserver(change);
const config = { attributes: true, childList: true, subtree: true };

let targetNodes = document.getElementsByClassName("dashboard-element table");

for (var i = 0; i < targetNodes.length; i++) {

    console.log(`observering: ${targetNodes[i]}`);

    observer.observe(targetNodes[i], config);
}

console.log("Observations complete");