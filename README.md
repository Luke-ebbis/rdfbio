# rdf-bio

Dealing with the conversion of biological formats and RDF encodings. See `rdfbio query --help` for more information.

```bash
# Fetch all omics resources related to cats and return the first 10 hits as RDF.
$ rdfbio query Cats -f ttl
<http://example.org/PXD024140> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/OmicDiDataSet> .
<http://example.org/PXD024140> <http://example.org/source> "pride" .
<http://example.org/PXD024140> <http://example.org/title> "Multi-omic analyses 1 in Abyssinian cats with primary renal amyloid deposits" .
<http://example.org/PXD017761> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/OmicDiDataSet> .
<http://example.org/PXD017761> <http://example.org/source> "pride" .
<http://example.org/PXD017761> <http://example.org/title> "Serum proteomics in cats with cardiomyopathy and congestive heart failure" .
<http://example.org/MTBLS440> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/OmicDiDataSet> .
<http://example.org/MTBLS440> <http://example.org/source> "metabolights_dataset" .
<http://example.org/MTBLS440> <http://example.org/title> "Untargeted metabolomic analysis in cats with naturally occurring inflammatory bowel disease and alimentary small cell lymphoma" .
<http://example.org/PXD033778> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/OmicDiDataSet> .
<http://example.org/PXD033778> <http://example.org/source> "pride" .
<http://example.org/PXD033778> <http://example.org/title> "Global proteomic profiling of multiple organs of cat (Felis catus) and proteome-transcriptome correlation during acute Toxoplasma gondii infection" .
<http://example.org/E-GEOD-49427> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/OmicDiDataSet> .
<http://example.org/E-GEOD-49427> <http://example.org/source> "biostudies-arrayexpress" .
<http://example.org/E-GEOD-49427> <http://example.org/title> "miRNA expression profiles in the serum of cats with hypertrophic cardiomyopathy compared to healthy cats" .
<http://example.org/GSE37177> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/OmicDiDataSet> .
<http://example.org/GSE37177> <http://example.org/source> "geo" .
<http://example.org/GSE37177> <http://example.org/title> "miRNA expression profiles in the serum of diabetic cats" .
<http://example.org/GSE49427> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/OmicDiDataSet> .
<http://example.org/GSE49427> <http://example.org/source> "geo" .
<http://example.org/GSE49427> <http://example.org/title> "miRNA expression profiles in the serum of cats with hypertrophic cardiomyopathy compared to healthy cats" .
<http://example.org/GSE121156> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/OmicDiDataSet> .
<http://example.org/GSE121156> <http://example.org/source> "geo" .
<http://example.org/GSE121156> <http://example.org/title> "Urinary miRNAs contained in exosome of dogs and cats" .
<http://example.org/PXD013663> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/OmicDiDataSet> .
<http://example.org/PXD013663> <http://example.org/source> "pride" .
<http://example.org/PXD013663> <http://example.org/title> "HDAC inhibition improves cardiopulmonary and mitochondrial function in a feline model with features of Heart Failure with Preserved Ejection Fraction" .
<http://example.org/GSE152946> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/OmicDiDataSet> .
<http://example.org/GSE152946> <http://example.org/source> "geo" .
<http://example.org/GSE152946> <http://example.org/title> "Developmental Genetics of Color Pattern Establishment in Cats" .
```
