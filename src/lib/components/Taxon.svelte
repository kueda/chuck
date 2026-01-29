<script lang="ts">
import { stripTags } from '$lib/utils/string';

interface Props {
  item: {
    taxonID?: string | null;
    scientificName?: string | null;
    taxonRank?: string | null;
    vernacularName?: string | null;
    taxonomicStatus?: string | null;
    higherClassification?: string | null;
    kingdom?: string | null;
    phylum?: string | null;
    class?: string | null;
    order?: string | null;
    superfamily?: string | null;
    family?: string | null;
    subfamily?: string | null;
    tribe?: string | null;
    subtribe?: string | null;
    genus?: string | null;
    subgenus?: string | null;
    infragenericEpithet?: string | null;
    specificEpithet?: string | null;
    infraspecificEpithet?: string | null;
    identificationVerificationStatus?: string | null;
    identificationCurrent?: boolean | null;
  };
}

const { item }: Props = $props();
const emoji = $derived.by(() => {
  // Idea from the amazing pyinat project, mappings adapted from
  // https://github.com/pyinat/pyinaturalist/blob/12fa880987811ba72dacce78c028302abcd798d1/pyinaturalist/docs/emoji.py.
  // Lowercasing because archives use inconsistent case for names
  const species = item.specificEpithet?.toLowerCase();
  if (species === 'canis familiaris') return '🐶';
  if (species === 'canis lupus') return '🐺';

  const genus = item.genus?.toLowerCase();
  if (genus === 'acer') return '🍁';
  if (genus === 'bison') return '🦬';
  if (genus === 'capra') return '🐐';
  if (genus === 'capsicum') return '🌶️';
  if (genus === 'cygnus') return '🦢';
  if (genus === 'fragaria') return '🍓';
  if (genus === 'gallus') return '🐔';
  if (genus === 'gorilla') return '🦍';
  if (genus === 'pavo') return '🦚';
  if (genus === 'persea') return '🥑';
  if (genus === 'prunus') return '🍒';
  if (genus === 'vulpes') return '🦊';

  const tribe = item.tribe?.toLowerCase();
  if (tribe === 'andropogoneae') return '🌽';
  if (tribe === 'camelini') return '🐪';
  if (tribe === 'cocoseae') return '🥥';
  if (tribe === 'maleae') return '🍎';

  const subfamily = item.subfamily?.toLowerCase();
  if (subfamily === 'antilopinae') return '🐑';
  if (subfamily === 'bovinae') return '🐮';
  if (subfamily === 'lutrinae') return '🦦';
  if (subfamily === 'pantherinae') return '🐯';
  if (subfamily === 'ponginae') return '🦧';
  if (subfamily === 'taxidiinae') return '🦡';
  if (subfamily === 'vaccinioideae') return '🫐';

  const family = item.family?.toLowerCase();
  if (family === 'actinidiaceae') return '🥝';
  if (family === 'altingiaceae') return '🌳';
  if (family === 'amaryllidaceae') return '🧄';
  if (family === 'anacardiaceae') return '🥭';
  if (family === 'apiaceae') return '🥕';
  if (family === 'asteraceae') return '🌻';
  if (family === 'brassicaceae') return '🥦';
  if (family === 'bromeliaceae') return '🍍';
  if (family === 'cactaceae') return '🌵';
  if (family === 'canidae') return '🐕';
  if (family === 'castoridae') return '🦫';
  if (family === 'cervidae') return '🦌';
  if (family === 'cornaceae') return '🌳';
  if (family === 'cucurbitaceae') return '🍈';
  if (family === 'delphinidae') return '🐬';
  if (family === 'equidae') return '🐴';
  if (family === 'fabaceae') return '🫛';
  if (family === 'felidae') return '🐈';
  if (family === 'formicidae') return '🐜';
  if (family === 'mephitidae') return '🦨';
  if (family === 'moraceae') return '🌳';
  if (family === 'musaceae') return '🍌';
  if (family === 'mustelidae') return '🦡';
  if (family === 'mutillidae') return '🐜';
  if (family === 'oleaceae') return '🫒';
  if (family === 'phascolarctidae') return '🐨';
  if (family === 'phasianidae') return '🦃';
  if (family === 'procyonidae') return '🦝';
  if (family === 'rosaceae') return '🌹';
  if (family === 'rutaceae') return '🍊';
  if (family === 'salicaceae') return '🌳';
  if (family === 'sciuridae') return '🐿️';
  if (family === 'solanaceae') return '🍅';
  if (family === 'ulmaceae') return '🌳';
  if (family === 'ursidae') return '🐻';

  const superfamily = item.superfamily?.toLowerCase();
  if (superfamily === 'coccinelloidea') return '🐞';
  if (superfamily === 'phocoidea') return '🦭';

  // Some of these I'm a little concerned about matching multiple things,
  // hence the lack of lowercasing. Might adjust with more experience
  if (item.higherClassification?.match(/Haplorhini/)) return '🐵';
  if (item.higherClassification?.match(/Serpentes/)) return '🐍';
  if (item.higherClassification?.match(/Suina/)) return '🐷';
  if (item.higherClassification?.match(/Tylopoda/)) return '🦙';

  if (item.higherClassification?.match(/Cetacea/)) return '🐋';
  if (item.higherClassification?.match(/Astacidea/)) return '🦞';
  if (item.higherClassification?.match(/Culicomorpha/)) return '🦟';

  const order = item.order?.toLowerCase();
  if (order === 'accipitriformes') return '🦅';
  if (order === 'anseriformes') return '🦆';
  if (order === 'arecales') return '🌴';
  if (order === 'blattodea') return '🪳';
  if (order === 'carnivora') return '🐆';
  if (order === 'chiroptera') return '🦇';
  if (order === 'coleoptera') return '🪲';
  if (order === 'columbiformes') return '🕊️';
  if (order === 'crocodylia') return '🐊';
  if (order === 'diptera') return '🪰';
  if (order === 'eulipotyphla') return '🦔';
  if (order === 'fagales') return '🌳';
  if (order === 'geraniales') return '🌺';
  if (order === 'hymenoptera') return '🐝';
  if (order === 'lagomorpha') return '🐰';
  if (order === 'lepidoptera') return '🦋';
  if (order === 'liliales') return '🌷';
  if (order === 'malvales') return '🌺';
  if (order === 'orthoptera') return '🦗';
  if (order === 'oxalidales') return '🍀';
  if (order === 'perciformes') return '🐠';
  if (order === 'phoenicopteriformes') return '🦩';
  if (order === 'pilosa') return '🦥';
  if (order === 'poales') return '🌾';
  if (order === 'proboscidea') return '🐘';
  if (order === 'pseudoscorpiones') return '🦂';
  if (order === 'psittaciformes') return '🦜';
  if (order === 'rodentia') return '🐹';
  if (order === 'scorpiones') return '🦂';
  if (order === 'sphenisciformes') return '🐧';
  if (order === 'strigiformes') return '🦉';
  if (order === 'testudines') return '🐢';
  if (order === 'tetraodontiformes') return '🐡';
  if (order === 'uropygi') return '🦂';
  if (order === 'vitales') return '🍇';

  if (item.higherClassification?.toLowerCase()?.match(/decapodiformes/))
    return '🦑';
  if (item.higherClassification?.toLowerCase()?.match(/peracarida/))
    return '🦐';

  if (item.higherClassification?.toLowerCase()?.match(/marsupialia/))
    return '🕷️';

  const klass = item.class?.toLowerCase();
  if (klass === 'actinopterygii') return '🐟';
  if (klass === 'amphibia') return '🐸';
  if (klass === 'anthozoa') return '🪸';
  if (klass === 'arachnida') return '🕷️';
  if (klass === 'aves') return '🐦';
  if (klass === 'cephalopoda') return '🐙';
  if (klass === 'chondrichthyes') return '🦈';
  if (klass === 'ginkgoopsida') return '🌳';
  if (klass === 'insecta') return '🐜';
  if (klass === 'malacostraca') return '🦀';
  if (klass === 'mammalia') return '🐀';
  if (klass === 'pinopsida') return '🌲';
  if (klass === 'polypodiopsida') return '🌿';
  if (klass === 'reptilia') return '🦎';

  if (item.higherClassification?.toLowerCase()?.match(/chelicerata/))
    return '🕷️';

  const phylum = item.phylum?.toLowerCase();
  if (phylum === 'mollusca') return '🐌';
  if (phylum === 'annelida') return '🪱';

  const kingdom = item.kingdom?.toLowerCase();
  if (kingdom === 'plantae') return '🌱';
  if (kingdom === 'fungi') return '🍄';
  if (kingdom === 'chromista') return '🟢';
  if (kingdom === 'bacteria') return '🦠';
  if (kingdom === 'viruses') return '🦠';

  return '';
});

const displaySciName = $derived.by(() => {
  if (!item.scientificName) return item.scientificName;
  const matches = item.scientificName?.match(/\(.+\)/i);
  // This is essentially uncontrolled input, so remove all potential tags
  let newName = stripTags(item.scientificName);
  matches?.forEach((match) => {
    newName = newName.replace(match, `<span>${match}</span>`);
  });
  return newName;
});
</script>

<span
  class={`taxon ${item.taxonRank?.toLowerCase()} ${item.vernacularName && item.scientificName ? 'both' : 'single'}`}
>
  {emoji}
  {#if item.vernacularName}
    <span class="vernacular">{item.vernacularName}</span>
  {/if}
  {#if displaySciName}
    <span class="scientific">
      {#if !item.taxonRank?.toString()?.match(/species/i)}
        <span class="rank">{item.taxonRank}</span>
        {@html displaySciName}
      {:else}
        {@html displaySciName}
      {/if}
    </span>
  {/if}
</span>

<style type="text/css">
  .both .scientific:before {
    content: '(';
  }
  .both .scientific:after {
    content: ')';
  }
  .genus .scientific,
  .species .scientific,
  .subspecies .scientific,
  .variety .scientific,
  .infraspecies .scientific {
    font-style: italic;
  }

  .rank {
    text-transform: capitalize;
    font-style: normal;
  }
  .scientific :global span {
    font-style: normal;
  }
</style>
