JSON.stringify((function () {
    const results = [];
    const items = document.querySelectorAll('li.arxiv-result');

    items.forEach(item => {
        try {
            const titleElement = item.querySelector('.title');
            const title = titleElement ? titleElement.textContent.replace('Title:', '').trim() : '';

            const authors = [];
            item.querySelectorAll('.authors > a').forEach(a => authors.push(a.textContent.trim()));

            const abstractElement = item.querySelector('.abstract-short');
            // Remove "Abstract" label and "Show Less" button text if present, simplified for now
            let summary = abstractElement ? abstractElement.textContent.replace('Abstract:', '').trim() : '';
            if (summary.endsWith('... ▽ More')) {
                summary = summary.replace('... ▽ More', '').trim();
            }

            const dateElement = item.querySelector('p.is-size-7');
            const dateText = dateElement ? dateElement.textContent : '';
            // Example: Submitted 30 October, 2023; originally announced October 2023.
            // Simplified extraction:
            const publishedDate = dateText.split(';')[0].replace('Submitted', '').trim();

            const linkElement = item.querySelector('.list-title > a');
            const url = linkElement ? linkElement.href : '';
            const id = url.split('/abs/')[1] || '';
            const pdfUrl = url.replace('/abs/', '/pdf/');

            results.push({
                id,
                title,
                authors,
                summary,
                published_date: publishedDate,
                url,
                pdf_url: pdfUrl,
                description_paragraphs: null
            });
        } catch (e) {
            console.error('Error parsing item', e);
        }
    });

    return results;
})())
