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
            // Remove "Abstract" label and "Show Less" button text if present.
            // abstract-short typically looks like: "... text ... ▽ More"
            let summary = abstractElement ? abstractElement.textContent.trim() : '';

            // Remove "Abstract:" prefix if it accidentally got included (usually failsafe)
            if (summary.startsWith('Abstract:')) {
                summary = summary.replace('Abstract:', '').trim();
            }

            // Remove trailing "More" link text
            // The arrow can be ▽ or &#9661; (down triangle)
            // We use a regex to be safe.
            summary = summary.replace(/\s*[▽▼]\s*More$/, '').trim();

            const dateElement = item.querySelector('p.is-size-7');
            const dateText = dateElement ? dateElement.textContent : '';
            // Example: "Submitted 30 October, 2023; originally announced October 2023."
            // We want "30 October, 2023"
            const publishedDate = dateText.split(';')[0].replace('Submitted', '').trim();

            const linkElement = item.querySelector('.list-title > a');
            const url = linkElement ? linkElement.href : '';
            // url example: https://arxiv.org/abs/2512.05073
            const id = url.split('/abs/')[1] || '';
            const pdfUrl = url.replace('/abs/', '/pdf/');

            if (id) {
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
            }
        } catch (e) {
            // console.error to stderr if possible, or just ignore bad items
        }
    });

    return results;
})())
