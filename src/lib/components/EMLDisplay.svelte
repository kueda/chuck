<script lang="ts">
import type { EMLData, EMLParty } from '$lib/utils/xmlParser';

interface Props {
  data: EMLData;
}

const { data }: Props = $props();
</script>

{#snippet party(party: EMLParty)}
  <h3 class="font-bold">{party.name}</h3>
  <table>
    <tbody>
      {#if party.electronicMailAddress && party.electronicMailAddress.length > 0}
        <tr>
          <th class="text-gray-400 pe-2">Email</th>
          <td>{party.electronicMailAddress.join(', ')}</td>
        </tr>
      {/if}
      {#if party.address && party.address.length > 0}
        <tr>
          <th class="text-gray-400 pe-2">Address</th>
          <td>{party.address.join(', ')}</td>
        </tr>
      {/if}
      {#if party.phone && party.phone.length > 0}
        <tr>
          <th class="text-gray-400 pe-2">Phone</th>
          <td>{party.phone.join(', ')}</td>
        </tr>
      {/if}
      {#if party.onlineUrl && party.onlineUrl.length > 0}
        <tr>
          <th class="text-gray-400 pe-2">Web page</th>
          <td>
            <ul>
            {#each party.onlineUrl as url}
              <li>
                <a href={url} target="_blank" class="text-ellipsis">{url}</a>
              </li>
            {/each}
            </ul>
          </td>
        </tr>
      {/if}
      {#if party.organizationName}
        <tr>
          <th class="text-gray-400 pe-2">Organization</th>
          <td>{party.organizationName}</td>
        </tr>
      {/if}
      {#if party.positionName}
        <tr>
          <th class="text-gray-400 pe-2">Position</th>
          <td>{party.positionName}</td>
        </tr>
      {/if}
    </tbody>
  </table>
{/snippet}

<div class="space-y-6">
  {#if data.title}
    <section>
      <h2 class="text-lg font-semibold mb-2">Dataset Title</h2>
      <p class="text-surface-700-300">{data.title}</p>
    </section>
  {/if}

  <dl class="flex flex-row flex-wrap gap-2 text-xs">
    {#if data.language}
      <div class="flex rounded overflow-hidden w-fit">
        <dt class="px-2 py-1 bg-gray-400">Language</dt>
        <dd class="px-2 py-1 bg-gray-300">{data.language}</dd>
      </div>
    {/if}
    {#if data.pubDate}
      <div class="flex rounded overflow-hidden w-fit">
        <dt class="px-2 py-1 bg-gray-400">Pub. Date</dt>
        <dd class="px-2 py-1 bg-gray-300">{data.pubDate}</dd>
      </div>
    {/if}
  </dl>

  <div class="grid grid-cols-3 gap-4">
    {#if data.creators && data.creators.length > 0}
      <section>
        <h2 class="text-lg font-semibold mb-2">Creators</h2>
        <ul class="overflow-hidden">
          {#each data.creators as creator}
            <li class="mb-3">
              {@render party(creator)}
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if data.metadataProviders && data.metadataProviders.length > 0}
      <section>
        <h2 class="text-lg font-semibold mb-2">Metadata Providers</h2>
        <ul>
          {#each data.metadataProviders as provider}
            <li class="mb-3">
              {@render party(provider)}
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if data.contact && data.contact.length > 0}
      <section>
        <h2 class="text-lg font-semibold mb-2">Contacts</h2>
        <ul>
          {#each data.contact as contact}
            <li class="mb-3">
              {@render party(contact)}
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  </div>

  {#if data.abstract && data.abstract.length > 0}
    <section>
      <h2 class="text-lg font-semibold mb-2">Abstract</h2>
      <div class="space-y-2">
        {#each data.abstract as paragraph}
          <p class="text-surface-700-300 whitespace-pre-wrap">{paragraph}</p>
        {/each}
      </div>
    </section>
  {/if}

  {#if data.keywords && data.keywords.length > 0}
    <section>
      <h2 class="text-lg font-semibold mb-2">Keywords</h2>
      <div class="flex flex-wrap gap-2">
        {#each data.keywords as keyword}
          <span class="bg-surface-200-800 px-2 py-1 rounded text-sm">
            {keyword}
          </span>
        {/each}
      </div>
    </section>
  {/if}

  {#if data.geographicCoverage}
    <section>
      <h2 class="text-lg font-semibold mb-2">Geographic Coverage</h2>
      <div class="space-y-1">
        {#if data.geographicCoverage.description}
          <p class="text-surface-700-300 mb-2">{data.geographicCoverage.description}</p>
        {/if}
        {#if data.geographicCoverage.north !== undefined}
          <p class="text-surface-700-300 text-sm">
            <span class="font-medium">North:</span> {data.geographicCoverage.north}°
          </p>
        {/if}
        {#if data.geographicCoverage.south !== undefined}
          <p class="text-surface-700-300 text-sm">
            <span class="font-medium">South:</span> {data.geographicCoverage.south}°
          </p>
        {/if}
        {#if data.geographicCoverage.east !== undefined}
          <p class="text-surface-700-300 text-sm">
            <span class="font-medium">East:</span> {data.geographicCoverage.east}°
          </p>
        {/if}
        {#if data.geographicCoverage.west !== undefined}
          <p class="text-surface-700-300 text-sm">
            <span class="font-medium">West:</span> {data.geographicCoverage.west}°
          </p>
        {/if}
      </div>
    </section>
  {/if}

  {#if data.additionalMetadata}
    <section>
      <h2 class="text-lg font-semibold mb-2">Additional Metadata</h2>
      <div class="space-y-1">
        {#if data.additionalMetadata.citation}
          <p class="text-surface-700-300 text-sm">
            <span class="font-medium">Citation:</span> {data.additionalMetadata.citation}
          </p>
        {/if}
        {#if data.additionalMetadata.livingTimePeriod}
          <p class="text-surface-700-300 text-sm">
            <span class="font-medium">Living Time Period:</span> {data.additionalMetadata.livingTimePeriod}
          </p>
        {/if}
        {#if data.additionalMetadata.hierarchyLevel}
          <p class="text-surface-700-300 text-sm">
            <span class="font-medium">Hierarchy Level:</span> {data.additionalMetadata.hierarchyLevel}
          </p>
        {/if}
        {#if data.additionalMetadata.dateStamp}
          <p class="text-surface-700-300 text-sm">
            <span class="font-medium">Date Stamp:</span> {data.additionalMetadata.dateStamp}
          </p>
        {/if}
        {#if data.additionalMetadata.resourceLogoUrl}
          <p class="text-surface-700-300 text-sm">
            <span class="font-medium">Resource Logo URL:</span> {data.additionalMetadata.resourceLogoUrl}
          </p>
        {/if}
        {#if data.additionalMetadata.physical}
          <p class="font-medium text-md mb-2">Physical Data</p>
          {#if data.additionalMetadata.physical.objectName}
            <p class="text-surface-700-300 text-sm">
              <span class="font-medium">Object Name:</span> {data.additionalMetadata.physical.objectName}
            </p>
          {/if}
          {#if data.additionalMetadata.physical.formatName}
            <p class="text-surface-700-300 text-sm">
              <span class="font-medium">Format:</span> {data.additionalMetadata.physical.formatName}
            </p>
          {/if}
          {#if data.additionalMetadata.physical.characterEncoding}
            <p class="text-surface-700-300 text-sm">
              <span class="font-medium">Character Encoding:</span> {data.additionalMetadata.physical.characterEncoding}
            </p>
          {/if}
          {#if data.additionalMetadata.physical.distributionUrl}
            <p class="text-surface-700-300 text-sm">
              <span class="font-medium">Distribution URL:</span> <a href={data.additionalMetadata.physical.distributionUrl} class="underline" target="_blank" rel="noopener noreferrer">{data.additionalMetadata.physical.distributionUrl}</a>
            </p>
          {/if}
        {/if}
      </div>
    </section>
  {/if}
</div>
