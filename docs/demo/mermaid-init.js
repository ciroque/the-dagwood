// Initialize Mermaid for mdBook
document.addEventListener('DOMContentLoaded', function() {
    // Configure Mermaid
    mermaid.initialize({
        startOnLoad: true,
        theme: 'dark',
        themeVariables: {
            primaryColor: '#1f2937',
            primaryTextColor: '#f3f4f6',
            primaryBorderColor: '#374151',
            lineColor: '#6b7280',
            secondaryColor: '#374151',
            tertiaryColor: '#4b5563'
        },
        flowchart: {
            useMaxWidth: true,
            htmlLabels: true,
            curve: 'basis'
        }
    });

    // Find all mermaid code blocks and render them
    const mermaidBlocks = document.querySelectorAll('code.language-mermaid');
    mermaidBlocks.forEach((block, index) => {
        // Create a div to hold the rendered diagram
        const div = document.createElement('div');
        div.className = 'mermaid';
        div.textContent = block.textContent;
        div.id = 'mermaid-diagram-' + index;
        
        // Replace the code block with the div
        block.parentNode.replaceChild(div, block);
    });

    // Re-initialize Mermaid to render the new diagrams
    mermaid.init();
});
