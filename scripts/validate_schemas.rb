# frozen_string_literal: true

require "open3"
require "pathname"

ROOT = Pathname(__dir__).parent.freeze
SCHEMA = ROOT / "schemas/v0/bhcp-v0.cddl"
EXAMPLES = ROOT / "schemas/v0/examples"
MANIFEST = EXAMPLES / "manifest.txt"
EXPECTED_KINDS = %w[
  canonical-ast semantic-ir syntax profile policy waiver extension-descriptor
  obligation-graph capability-graph state-graph execution-graph evidence-bundle
  runtime-outcome planner-request planner-result feature-manifest content-reference
].freeze

def run!(*command, stdin_data: nil)
  stdout, stderr, status = Open3.capture3(*command, stdin_data: stdin_data)
  return stdout if status.success?

  warn stdout unless stdout.empty?
  warn stderr unless stderr.empty?
  abort "command failed (#{status.exitstatus}): #{command.join(' ')}"
end

def executable!(name)
  path = ENV.fetch("PATH").split(File::PATH_SEPARATOR)
            .map { |dir| File.join(dir, name) }
            .find { |candidate| File.file?(candidate) && File.executable?(candidate) }
  abort "missing executable from pinned bundle: #{name}" unless path
  path
end

cddl = executable!("cddl")
diag2cbor = executable!("diag2cbor.rb")
cbor2diag = executable!("cbor2diag.rb")
cbor2cbor = executable!("cbor2cbor.rb")

abort "missing schema: #{SCHEMA}" unless SCHEMA.file?
abort "missing fixture manifest: #{MANIFEST}" unless MANIFEST.file?

entries = MANIFEST.readlines(chomp: true).reject(&:empty?).map do |line|
  file, kind, extra = line.split
  abort "invalid manifest line: #{line}" if !file || !kind || extra
  [file, kind]
end

kinds = entries.map(&:last)
abort "root fixture kinds do not match schema inventory" unless kinds.sort == EXPECTED_KINDS.sort
abort "duplicate fixture kind" unless kinds.uniq.length == kinds.length

entries.each do |file, expected_kind|
  diagnostic = EXAMPLES / file
  abort "missing fixture: #{diagnostic}" unless diagnostic.file?

  bytes = run!(diag2cbor, diagnostic.to_s)
  deterministic_bytes = run!(cbor2cbor, "-d", stdin_data: bytes)
  temporary = EXAMPLES / ".#{diagnostic.basename}.cbor"
  begin
    temporary.binwrite(deterministic_bytes)
    run!(cddl, SCHEMA.to_s, "validate", temporary.to_s)

    normalized_diag = run!(cbor2diag, temporary.to_s)
    actual_kind = normalized_diag[/"kind"\s*:\s*"([^"]+)"/, 1]
    abort "#{file}: expected kind #{expected_kind}, found #{actual_kind.inspect}" unless actual_kind == expected_kind

    # Diagnostic -> deterministic CBOR -> diagnostic -> deterministic CBOR must be stable.
    second_bytes = run!(diag2cbor, stdin_data: normalized_diag)
    second_deterministic_bytes = run!(cbor2cbor, "-d", stdin_data: second_bytes)
    unless second_deterministic_bytes == deterministic_bytes
      abort "#{file}: deterministic CBOR round trip changed bytes"
    end

    # The universal digest has a fixed wire size. This lexical check is additional to CDDL.
    diagnostic.read.scan(/"algorithm"\s*:\s*"sha2-256"\s*,\s*"digest"\s*:\s*h'([0-9a-fA-F]*)'/) do |hex|
      abort "#{file}: sha2-256 digest must be 32 bytes" unless hex.first.length == 64
    end
  ensure
    temporary.delete if temporary.exist?
  end
end

puts "validated #{entries.length} BHCP v0 root documents with deterministic CBOR round trips"
