<script lang="ts">
import DOMPurify from 'dompurify';
import MarkdownIt from 'markdown-it';

// Ensure all links open in an external browser
DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.nodeName === 'A') {
    node.setAttribute('target', '_blank');
    node.setAttribute('rel', 'noopener noreferrer');
  }

  // Override tailwind's weird base styles for lists
  const listClass = 'ps-6';
  if (node.nodeName === 'OL') {
    node.setAttribute('class', `${listClass} list-decimal`);
    node.setAttribute('role', 'list');
  }
  if (node.nodeName === 'UL') {
    node.setAttribute('class', `${listClass} list-disc`);
    node.setAttribute('role', 'list');
  }
});

const { text }: { text: string } = $props();

// Configure markdown parser
const md = new MarkdownIt({
  html: true,
  breaks: true,
  linkify: true,
});

const displayText = $derived.by(() => {
  if (text) return md.render(text);
  return text;
});
</script>

{@html DOMPurify.sanitize(displayText)}
