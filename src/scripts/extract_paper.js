JSON.stringify((function () {
    try {
        const titleElement = document.querySelector('h1.title');
        const title = titleElement ? titleElement.textContent.replace('Title:', '').trim() : '';

        const authors = [];
        document.querySelectorAll('.authors > a').forEach(a => authors.push(a.textContent.trim()));

        const abstractElement = document.querySelector('blockquote.abstract');
        const summary = abstractElement ? abstractElement.textContent.replace('Abstract:', '').trim() : '';

        const dateElement = document.querySelector('.dateline');
        // Example: (Submitted on 17 Jun 2017)
        const publishedDate = dateElement ? dateElement.textContent.replace('(Submitted on', '').replace(')', '').trim() : '';

        // URL from window location or link
        const url = window.location.href;
        const id = url.split('/abs/')[1] ? url.split('/abs/')[1].split('v')[0] : ''; // Handle versions if necessary
        const pdfUrl = url.replace('/abs/', '/pdf/');

        return {
            id,
            title,
            authors,
            summary,
            published_date: publishedDate,
            url,
            pdf_url: pdfUrl,
            description_paragraphs: null
        };
    } catch (e) {
        console.error('Error extracting paper', e);
        return null;
    }
})())
