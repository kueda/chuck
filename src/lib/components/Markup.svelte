<script lang="ts">
  import MarkdownIt from "markdown-it";
  import DOMPurify from 'dompurify';

  // Ensure all links open in an external browser
  DOMPurify.addHook('afterSanitizeAttributes', node => {
    if (node.nodeName === 'A') {
      node.setAttribute('target', '_blank');
      node.setAttribute('rel', 'noopener noreferrer');
    }
  });

  let { text }: { text: string } = $props();

  // Configure markdown parser
  const md = new MarkdownIt({
    html: true,
    breaks: true,
    linkify: true,
  });

  let displayText = $derived.by(() => {
    if (text) return md.render(text);
    return text;
  });
</script>

{@html DOMPurify.sanitize(displayText)}
