function change(mutationList, observer) {
    for (var i = 0; i < mutationList.length; i++) {
if (mutationList[i].target.nodeName == "TD" && mutationList[i].target.classList.contains("string") && !mutationList[i].target.classList.contains("ssphp_modified")) {
         mutationList[i].target.innerHTML = mutationList[i].target.innerHTML.replace(/~~(.*?)~~(.*?)~~/g, '<span class="$2">$1</span>');
         mutationList[i].target.innerHTML = mutationList[i].target.innerHTML.replace(/~!(.*?)~!(.*?)~!(.*?)~!/g, '<$1>$2<$3>');
            mutationList[i].target.classList.add("ssphp_modified");    
}
    }
};
const observer = new MutationObserver(change);

const config = { attributes: true, childList: true, subtree: true };

let targetNodes = document.getElementsByClassName("dashboard-element table");

for (var i=0; i < targetNodes.length; i++) {
    observer.observe(targetNodes[i], config);
}