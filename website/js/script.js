document.addEventListener('DOMContentLoaded', () => {
    const gallery = document.querySelector('.gallery');
    if (gallery) {
        fetch('https://images-api.nasa.gov/search?q=space&media_type=image')
            .then(res => res.json())
            .then(data => {
                const items = data.collection.items.slice(0, 6);
                for (const item of items) {
                    const img = document.createElement('img');
                    img.src = item.links[0].href;
                    img.alt = item.data[0].title;
                    gallery.appendChild(img);
                }
            })
            .catch(err => console.error('Failed to load images', err));
    }
});
