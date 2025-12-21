(function () {
    if (document.querySelector('li.arxiv-result')) return 'found';
    if (document.body && document.body.innerText.includes('Sorry, your query returned no results')) return 'empty';
    return null;
})()
