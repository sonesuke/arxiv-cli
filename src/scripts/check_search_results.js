(function () {
    if (document.querySelector('li.arxiv-result')) return 'found';
    if (document.body && document.body.innerText.includes('Sorry, your query returned no results')) return 'empty';
    if (document.querySelector('h1.title')) return 'abstract';
    return null;
})()
