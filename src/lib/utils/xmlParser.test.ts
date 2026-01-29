import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { parseEML, parseMeta } from './xmlParser';

describe('parseEML', () => {
  it('extracts dataset title', () => {
    const xml = `<?xml version="1.0"?>
<eml:eml xmlns:eml="eml://ecoinformatics.org/eml-2.1.1">
  <dataset>
    <title>My Test Dataset</title>
  </dataset>
</eml:eml>`;

    const result = parseEML(xml);
    expect(result.title).toBe('My Test Dataset');
  });

  it('extracts creators', () => {
    const xml = `<?xml version="1.0"?>
<eml:eml xmlns:eml="eml://ecoinformatics.org/eml-2.1.1">
  <dataset>
    <creator>
      <individualName>
        <givenName>John</givenName>
        <surName>Doe</surName>
      </individualName>
      <electronicMailAddress>john@example.org</electronicMailAddress>
    </creator>
    <creator>
      <individualName>
        <givenName>Jane</givenName>
        <surName>Smith</surName>
      </individualName>
    </creator>
  </dataset>
</eml:eml>`;

    const result = parseEML(xml);
    expect(result.creators).toHaveLength(2);
    expect(result.creators[0].name).toBe('John Doe');
    expect(result.creators[0].email).toBe('john@example.org');
    expect(result.creators[1].name).toBe('Jane Smith');
    expect(result.creators[1].email).toBeUndefined();
  });

  it('extracts abstract paragraphs', () => {
    const xml = `<?xml version="1.0"?>
<eml:eml xmlns:eml="eml://ecoinformatics.org/eml-2.1.1">
  <dataset>
    <abstract>
      <para>This is the first paragraph.</para>
      <para>This is the second paragraph.</para>
    </abstract>
  </dataset>
</eml:eml>`;

    const result = parseEML(xml);
    expect(result.abstract).toHaveLength(2);
    expect(result.abstract?.[0]).toBe('This is the first paragraph.');
    expect(result.abstract?.[1]).toBe('This is the second paragraph.');
  });

  it('handles missing optional fields', () => {
    const xml = `<?xml version="1.0"?>
<eml:eml xmlns:eml="eml://ecoinformatics.org/eml-2.1.1">
  <dataset>
    <title>Minimal Dataset</title>
  </dataset>
</eml:eml>`;

    const result = parseEML(xml);
    expect(result.title).toBe('Minimal Dataset');
    expect(result.creators).toEqual([]);
    expect(result.abstract).toBeUndefined();
  });

  it('returns null for invalid XML', () => {
    const result = parseEML('not valid xml');
    expect(result).toBeNull();
  });

  describe('with fixture data', () => {
    const fixtureXml = readFileSync(
      join(__dirname, '../../../tests/fixtures/eml.xml'),
      'utf-8',
    );

    it('extracts creator information', () => {
      const result = parseEML(fixtureXml);
      expect(result).not.toBeNull();
      expect(result?.creators).toHaveLength(1);
      expect(result?.creators[0].name).toBe('Jane Smith');
      expect(result?.creators[0].email).toBe('jsmith@example.edu');
    });

    it('extracts metadataProvider information', () => {
      const result = parseEML(fixtureXml);
      expect(result).not.toBeNull();
      expect(result?.metadataProviders).toBeDefined();
      expect(result?.metadataProviders).toHaveLength(1);
      expect(result?.metadataProviders?.[0].name).toBe('Jay Blue');
      expect(result?.metadataProviders?.[0].email).toBe('jblue@example.edu');
    });

    it('extracts language', () => {
      const result = parseEML(fixtureXml);
      expect(result).not.toBeNull();
      expect(result?.language).toBe('eng');
    });

    it('extracts pubDate', () => {
      const result = parseEML(fixtureXml);
      expect(result).not.toBeNull();
      expect(result?.pubDate).toBe('2020-01-15');
    });

    it('extracts contact information', () => {
      const result = parseEML(fixtureXml);
      expect(result).not.toBeNull();
      expect(result?.contact).toBeDefined();
      expect(result?.contact).toHaveLength(1);
      expect(result?.contact?.[0].name).toBe('Jane Smith');
      expect(result?.contact?.[0].email).toBe('jsmith@example.edu');
    });
  });
});

describe('parseMeta', () => {
  it('extracts core file information', () => {
    const xml = `<?xml version="1.0"?>
<archive>
  <core rowType="http://rs.tdwg.org/dwc/terms/Occurrence">
    <files>
      <location>occurrence.csv</location>
    </files>
    <id index="0" />
    <field index="0" term="http://rs.gbif.org/terms/1.0/gbifID"/>
    <field index="1" term="http://rs.tdwg.org/dwc/terms/scientificName"/>
  </core>
</archive>`;

    const result = parseMeta(xml);
    expect(result?.core.rowType).toBe('Occurrence');
    expect(result?.core.location).toBe('occurrence.csv');
    expect(result?.core.idIndex).toBe(0);
    expect(result?.core.fields).toHaveLength(2);
    expect(result?.core.fields[0].index).toBe(0);
    expect(result?.core.fields[0].term).toBe(
      'http://rs.gbif.org/terms/1.0/gbifID',
    );
  });

  it('extracts extension information', () => {
    const xml = `<?xml version="1.0"?>
<archive>
  <core>
    <files><location>occurrence.csv</location></files>
  </core>
  <extension rowType="http://rs.gbif.org/terms/1.0/Multimedia">
    <files>
      <location>multimedia.csv</location>
    </files>
    <coreid index="0" />
  </extension>
</archive>`;

    const result = parseMeta(xml);
    expect(result?.extensions).toHaveLength(1);
    expect(result?.extensions[0].rowType).toBe('Multimedia');
    expect(result?.extensions[0].location).toBe('multimedia.csv');
  });

  it('returns null for invalid XML', () => {
    const result = parseMeta('not valid xml');
    expect(result).toBeNull();
  });
});
