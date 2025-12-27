export interface EMLAddress {
  deliveryPoint?: string[];
  city?: string;
  administrativeArea?: string;
  postalCode?: string;
  country?: string;
}

export interface EMLParty {
  // Person name fields
  individualName?: {
    salutation?: string[];
    givenName?: string[];
    surName: string;
  };
  organizationName?: string;
  positionName?: string;

  // Contact information
  address?: EMLAddress[];
  phone?: string[];
  electronicMailAddress?: string[];
  onlineUrl?: string[];
  userId?: Array<{
    value: string;
    directory: string;
  }>;

  // Computed fields for convenience
  name: string; // Computed from individualName or organizationName or positionName
  email?: string; // First electronicMailAddress for backwards compatibility
}

export interface EMLPhysical {
  objectName?: string;
  characterEncoding?: string;
  formatName?: string;
  distributionUrl?: string;
}

export interface EMLAdditionalMetadata {
  dateStamp?: string;
  hierarchyLevel?: string;
  citation?: string;
  resourceLogoUrl?: string;
  livingTimePeriod?: string;
  physical?: EMLPhysical;
}

export interface EMLData {
  title?: string;
  creators: EMLParty[];
  metadataProviders?: EMLParty[];
  contact?: EMLParty[];
  abstract?: string[];
  language?: string;
  pubDate?: string;
  keywords?: string[];
  geographicCoverage?: {
    description?: string;
    north?: number;
    south?: number;
    east?: number;
    west?: number;
  };
  additionalMetadata?: EMLAdditionalMetadata;
}

export interface MetaField {
  index: number;
  term: string;
  default?: string;
  vocabulary?: string;
}

export interface MetaCore {
  rowType: string;
  location: string;
  idIndex?: number;
  fields: MetaField[];
}

export interface MetaExtension {
  rowType: string;
  location: string;
  coreIdIndex?: number;
  fields: MetaField[];
}

export interface MetaData {
  core: MetaCore;
  extensions: MetaExtension[];
}

/**
 * Parse a ResponsibleParty element (creator, metadataProvider, contact, etc.)
 */
function parseParty(partyEl: Element): EMLParty | null {
  // Extract individualName
  let individualName: EMLParty['individualName'];
  const individualNameEl = partyEl.querySelector('individualName');
  if (individualNameEl) {
    const salutations: string[] = [];
    individualNameEl.querySelectorAll('salutation').forEach(el => {
      const text = el.textContent?.trim();
      if (text) salutations.push(text);
    });

    const givenNames: string[] = [];
    individualNameEl.querySelectorAll('givenName').forEach(el => {
      const text = el.textContent?.trim();
      if (text) givenNames.push(text);
    });

    const surName = individualNameEl.querySelector('surName')?.textContent?.trim();

    if (surName) {
      individualName = {
        surName,
        salutation: salutations.length > 0 ? salutations : undefined,
        givenName: givenNames.length > 0 ? givenNames : undefined,
      };
    }
  }

  // Extract organizationName
  const organizationName = partyEl.querySelector('organizationName')?.textContent?.trim();

  // Extract positionName
  const positionName = partyEl.querySelector('positionName')?.textContent?.trim();

  // Extract addresses
  const addresses: EMLAddress[] = [];
  partyEl.querySelectorAll('address').forEach(addressEl => {
    const deliveryPoints: string[] = [];
    addressEl.querySelectorAll('deliveryPoint').forEach(el => {
      const text = el.textContent?.trim();
      if (text) deliveryPoints.push(text);
    });

    const address: EMLAddress = {
      deliveryPoint: deliveryPoints.length > 0 ? deliveryPoints : undefined,
      city: addressEl.querySelector('city')?.textContent?.trim(),
      administrativeArea: addressEl.querySelector('administrativeArea')?.textContent?.trim(),
      postalCode: addressEl.querySelector('postalCode')?.textContent?.trim(),
      country: addressEl.querySelector('country')?.textContent?.trim(),
    };

    addresses.push(address);
  });

  // Extract phones
  const phones: string[] = [];
  partyEl.querySelectorAll('phone').forEach(el => {
    const text = el.textContent?.trim();
    if (text) phones.push(text);
  });

  // Extract emails
  const emails: string[] = [];
  partyEl.querySelectorAll('electronicMailAddress').forEach(el => {
    const text = el.textContent?.trim();
    if (text) emails.push(text);
  });

  // Extract URLs
  const urls: string[] = [];
  partyEl.querySelectorAll('onlineUrl').forEach(el => {
    const text = el.textContent?.trim();
    if (text) urls.push(text);
  });

  // Extract userIds
  const userIds: Array<{ value: string; directory: string }> = [];
  partyEl.querySelectorAll('userId').forEach(el => {
    const value = el.textContent?.trim();
    const directory = el.getAttribute('directory');
    if (value && directory) {
      userIds.push({ value, directory });
    }
  });

  // Compute name field for backwards compatibility
  let name = '';
  if (individualName) {
    const givenNameStr = individualName.givenName?.join(' ') || '';
    name = [givenNameStr, individualName.surName].filter(Boolean).join(' ');
  } else if (organizationName) {
    name = organizationName;
  } else if (positionName) {
    name = positionName;
  }

  if (!name) {
    return null;
  }

  return {
    individualName,
    organizationName,
    positionName,
    address: addresses.length > 0 ? addresses : undefined,
    phone: phones.length > 0 ? phones : undefined,
    electronicMailAddress: emails.length > 0 ? emails : undefined,
    onlineUrl: urls.length > 0 ? urls : undefined,
    userId: userIds.length > 0 ? userIds : undefined,
    name,
    email: emails.length > 0 ? emails[0] : undefined,
  };
}

/**
 * Parse EML XML and extract structured metadata
 */
export function parseEML(xml: string): EMLData | null {
  try {
    const parser = new DOMParser();
    const doc = parser.parseFromString(xml, 'text/xml');

    // Check for parse errors
    if (doc.querySelector('parsererror')) {
      return null;
    }

    const dataset = doc.querySelector('dataset');
    if (!dataset) {
      return null;
    }

    // Extract title
    const titleEl = dataset.querySelector('title');
    const title = titleEl?.textContent?.trim();

    // Extract creators
    const creators: EMLParty[] = [];
    const creatorEls = dataset.querySelectorAll('creator');
    creatorEls.forEach((creator) => {
      const party = parseParty(creator);
      if (party) {
        creators.push(party);
      }
    });

    // Extract metadataProviders
    const metadataProviders: EMLParty[] = [];
    const metadataProviderEls = dataset.querySelectorAll('metadataProvider');
    metadataProviderEls.forEach((provider) => {
      const party = parseParty(provider);
      if (party) {
        metadataProviders.push(party);
      }
    });

    // Extract contact
    const contact: EMLParty[] = [];
    const contactEls = dataset.querySelectorAll('contact');
    contactEls.forEach((contactEl) => {
      const party = parseParty(contactEl);
      if (party) {
        contact.push(party);
      }
    });

    // Extract abstract (all paragraphs)
    const abstractParagraphs: string[] = [];
    const abstractEl = dataset.querySelector('abstract');
    if (abstractEl) {
      const paraEls = abstractEl.querySelectorAll('para');
      paraEls.forEach((para) => {
        const text = para.textContent?.trim();
        if (text) abstractParagraphs.push(text);
      });
    }

    // Extract language
    const languageEl = dataset.querySelector('language');
    const language = languageEl?.textContent?.trim();

    // Extract pubDate
    const pubDateEl = dataset.querySelector('pubDate');
    const pubDate = pubDateEl?.textContent?.trim();

    // Extract keywords
    const keywords: string[] = [];
    const keywordEls = dataset.querySelectorAll('keyword');
    keywordEls.forEach((kw) => {
      const text = kw.textContent?.trim();
      if (text) keywords.push(text);
    });

    // Extract geographic coverage
    let geographicCoverage;
    const geoCoverage = dataset.querySelector('coverage geographicCoverage');
    if (geoCoverage) {
      const description = geoCoverage.querySelector('geographicDescription')?.textContent?.trim();
      const north = parseFloat(geoCoverage.querySelector('northBoundingCoordinate')?.textContent || '');
      const south = parseFloat(geoCoverage.querySelector('southBoundingCoordinate')?.textContent || '');
      const east = parseFloat(geoCoverage.querySelector('eastBoundingCoordinate')?.textContent || '');
      const west = parseFloat(geoCoverage.querySelector('westBoundingCoordinate')?.textContent || '');

      geographicCoverage = {
        description: description || undefined,
        north: !isNaN(north) ? north : undefined,
        south: !isNaN(south) ? south : undefined,
        east: !isNaN(east) ? east : undefined,
        west: !isNaN(west) ? west : undefined,
      };
    }

    // Extract additionalMetadata (outside dataset element)
    let additionalMetadata;
    const additionalMetadataEl = doc.querySelector('additionalMetadata metadata gbif');
    if (additionalMetadataEl) {
      const dateStamp = additionalMetadataEl.querySelector('dateStamp')?.textContent?.trim();
      const hierarchyLevel = additionalMetadataEl.querySelector('hierarchyLevel')?.textContent?.trim();
      const citation = additionalMetadataEl.querySelector('citation')?.textContent?.trim();
      const resourceLogoUrl = additionalMetadataEl.querySelector('resourceLogoUrl')?.textContent?.trim();
      const livingTimePeriod = additionalMetadataEl.querySelector('livingTimePeriod')?.textContent?.trim();

      // Extract physical and distribution
      let physical;
      const physicalEl = additionalMetadataEl.querySelector('physical');
      if (physicalEl) {
        const objectName = physicalEl.querySelector('objectName')?.textContent?.trim();
        const characterEncoding = physicalEl.querySelector('characterEncoding')?.textContent?.trim();
        const formatName = physicalEl.querySelector('dataFormat externallyDefinedFormat formatName')?.textContent?.trim();
        const distributionUrl = physicalEl.querySelector('distribution online url')?.textContent?.trim();

        if (objectName || characterEncoding || formatName || distributionUrl) {
          physical = {
            objectName: objectName || undefined,
            characterEncoding: characterEncoding || undefined,
            formatName: formatName || undefined,
            distributionUrl: distributionUrl || undefined,
          };
        }
      }

      if (dateStamp || hierarchyLevel || citation || resourceLogoUrl || livingTimePeriod || physical) {
        additionalMetadata = {
          dateStamp: dateStamp || undefined,
          hierarchyLevel: hierarchyLevel || undefined,
          citation: citation || undefined,
          resourceLogoUrl: resourceLogoUrl || undefined,
          livingTimePeriod: livingTimePeriod || undefined,
          physical: physical || undefined,
        };
      }
    }

    return {
      title: title || undefined,
      creators,
      metadataProviders: metadataProviders.length > 0 ? metadataProviders : undefined,
      contact: contact.length > 0 ? contact : undefined,
      abstract: abstractParagraphs.length > 0 ? abstractParagraphs : undefined,
      language: language || undefined,
      pubDate: pubDate || undefined,
      keywords: keywords.length > 0 ? keywords : undefined,
      geographicCoverage,
      additionalMetadata,
    };
  } catch (e) {
    console.error('Failed to parse EML:', e);
    return null;
  }
}

export function metaFieldsFromElements(fieldEls: NodeListOf<Element>) {
  const fields: MetaField[] = [];

  // This weird approach probably isn't necessary but sometimes GBIF has
  // multiple sibling elements for the same term and different other
  // attributes
  const defaults: any = {};
  const vocabs: any = {};
  fieldEls.forEach(field => {
    const term = field.getAttribute('term') || '';
    const defaultVal = field.getAttribute('default') || '';
    const vocabulary = field.getAttribute('vocabulary') || '';
    if (term && defaultVal) defaults[term] = defaultVal;
    if (term && vocabulary) vocabs[term] = vocabulary;
  });
  fieldEls.forEach(field => {
    const index = parseInt(field.getAttribute('index') || '');
    const term = field.getAttribute('term') || '';
    if (!isNaN(index) && term) {
      fields.push({
        index,
        term,
      default: defaults[term],
      vocabulary: vocabs[term]
    });
    }
  });
  return fields;
}

/**
 * Parse meta.xml and extract archive structure
 */
export function parseMeta(xml: string): MetaData | null {
  try {
    const parser = new DOMParser();
    const doc = parser.parseFromString(xml, 'text/xml');

    // Check for parse errors
    if (doc.querySelector('parsererror')) {
      return null;
    }

    const archive = doc.querySelector('archive');
    if (!archive) {
      return null;
    }

    // Extract core information
    const coreEl = archive.querySelector('core');
    if (!coreEl) {
      return null;
    }

    const rowType = extractTermName(coreEl.getAttribute('rowType') || '');
    const location = coreEl.querySelector('location')?.textContent?.trim() || '';
    const idIndex = parseInt(coreEl.querySelector('id')?.getAttribute('index') || '');
    const fieldEls = coreEl.querySelectorAll('field');

    const core: MetaCore = {
      rowType,
      location,
      idIndex: !isNaN(idIndex) ? idIndex : undefined,
      fields: metaFieldsFromElements(fieldEls),
    };

    // Extract extensions
    const extensions: MetaExtension[] = [];
    const extensionEls = archive.querySelectorAll('extension');
    extensionEls.forEach((extEl) => {
      const rowType = extractTermName(extEl.getAttribute('rowType') || '');
      const location = extEl.querySelector('location')?.textContent?.trim() || '';
      const coreIdIndex = parseInt(extEl.querySelector('coreid')?.getAttribute('index') || '');
      const fieldEls = extEl.querySelectorAll('field');

      extensions.push({
        rowType,
        location,
        coreIdIndex: !isNaN(coreIdIndex) ? coreIdIndex : undefined,
        fields: metaFieldsFromElements(fieldEls),
      });
    });

    return { core, extensions };
  } catch (e) {
    console.error('Failed to parse meta.xml:', e);
    return null;
  }
}

/**
 * Extract the term name from a full URI
 * e.g., "http://rs.tdwg.org/dwc/terms/Occurrence" -> "Occurrence"
 */
function extractTermName(uri: string): string {
  if (!uri) return '';
  const parts = uri.split('/');
  const lastPart = parts[parts.length - 1];
  // Also handle terms with # separator
  const hashParts = lastPart.split('#');
  return hashParts[hashParts.length - 1];
}

// Works fine except it doesn't indent attributes... which doesn't seem to be
// something most 3rd parties support either.
// 
// https://stackoverflow.com/a/47317538
export function prettify(xml: string) {
  const xmlDoc = new DOMParser().parseFromString(xml, 'application/xml');
  const xsltDoc = new DOMParser().parseFromString([
    // describes how we want to modify the XML - indent everything
    '<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform">',
    '  <xsl:strip-space elements="*"/>',
    '  <xsl:template match="para[content-style][not(text())]">', // change to just text() to strip space in text nodes
    '    <xsl:value-of select="normalize-space(.)"/>',
    '  </xsl:template>',
    '  <xsl:template match="node()|@*">',
    '    <xsl:copy><xsl:apply-templates select="node()|@*"/></xsl:copy>',
    '  </xsl:template>',
    '  <xsl:output indent="yes"/>',
    '</xsl:stylesheet>',
  ].join('\n'), 'application/xml');

  const xsltProcessor = new XSLTProcessor();
  xsltProcessor.importStylesheet(xsltDoc);
  return new XMLSerializer().serializeToString(
    xsltProcessor.transformToDocument(xmlDoc)
  );
}
