(function () {
    function currentTheme() {
        return document.documentElement.classList.contains('light') ? 'light' : 'dark';
    }

    function updateButton() {
        var btn = document.getElementById('theme_toggle_btn');
        if (btn) btn.textContent = currentTheme() === 'light' ? '🌙' : '☀️';
    }

    window.toggleTheme = function () {
        var isLight = document.documentElement.classList.toggle('light');
        try { localStorage.setItem('theme', isLight ? 'light' : 'dark'); } catch (e) {}
        updateButton();
    };

    document.addEventListener('DOMContentLoaded', updateButton);
    document.addEventListener('htmx:afterSettle', updateButton);
})();