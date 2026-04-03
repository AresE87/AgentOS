import { useCallback, useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import {
  ArrowUpRight,
  BadgeCheck,
  Building2,
  FolderOpen,
  Globe2,
  LineChart,
  Rocket,
  ShieldEllipsis,
} from 'lucide-react';
import Card from '../../components/Card';
import Button from '../../components/Button';
import EmptyState from '../../components/EmptyState';
import { useAgent } from '../../hooks/useAgent';

interface Partner {
  id: string;
  company: string;
  device_type: string;
  integration_level: 'basic' | 'premium' | 'exclusive';
  certified: boolean;
  certification_note?: string | null;
  certification_evidence?: string | null;
  registered_at: string;
  certified_at?: string | null;
}

interface InvestorMetrics {
  arr: number;
  mrr_growth_pct: number;
  gross_margin: number;
  burn_rate: number;
  runway_months: number;
  total_users: number;
  paid_users: number;
  ltv_cac_ratio: number;
  modeled?: boolean;
  source_note?: string;
}

interface RegionStatus {
  region: string;
  status: string;
  latency_ms: number;
  last_checked: string;
  probe_type?: string;
  note?: string;
}

interface InfraStatus {
  regions: RegionStatus[];
  global_status: string;
  uptime_pct: number;
  probe_mode?: string;
  source_note?: string;
}

interface DataRoomDocument {
  name: string;
  category: string;
  description: string;
  status: string;
  path?: string | null;
  last_modified?: string | null;
  source_type?: string;
}

interface YearProjection {
  year: number;
  arr: number;
  users: number;
  revenue: number;
  costs: number;
  modeled_note?: string | null;
}

interface RepoArtifact {
  name: string;
  path: string;
  status: string;
  last_modified?: string | null;
  source_type?: string;
}

interface ReadinessArtifacts {
  demo_tracks: string[];
  evidence_docs: RepoArtifact[];
  market_readiness?: RepoArtifact | null;
  definitive_mode?: RepoArtifact | null;
}

export default function Readiness() {
  const {
    listPartners,
    registerPartner,
    certifyPartner,
    getInvestorMetrics,
    getInfraStatus,
    getDataRoom,
    getFinancialProjections,
    getReadinessArtifacts,
  } = useAgent();

  const [partners, setPartners] = useState<Partner[]>([]);
  const [metrics, setMetrics] = useState<InvestorMetrics | null>(null);
  const [infra, setInfra] = useState<InfraStatus | null>(null);
  const [dataRoom, setDataRoom] = useState<DataRoomDocument[]>([]);
  const [projections, setProjections] = useState<YearProjection[]>([]);
  const [artifacts, setArtifacts] = useState<ReadinessArtifacts | null>(null);
  const [company, setCompany] = useState('');
  const [deviceType, setDeviceType] = useState('');
  const [integrationLevel, setIntegrationLevel] = useState<'basic' | 'premium' | 'exclusive'>('basic');
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setBusy('refresh');
    setError(null);

    try {
      const [partnersResult, metricsResult, infraResult, dataRoomResult, projectionsResult, artifactsResult] = await Promise.all([
        listPartners().catch(() => []),
        getInvestorMetrics().catch(() => null),
        getInfraStatus().catch(() => null),
        getDataRoom().catch(() => []),
        getFinancialProjections(3).catch(() => []),
        getReadinessArtifacts().catch(() => null),
      ]);

      setPartners(Array.isArray(partnersResult) ? partnersResult : []);
      setMetrics(metricsResult);
      setInfra(infraResult);
      setDataRoom(Array.isArray(dataRoomResult) ? dataRoomResult : []);
      setProjections(Array.isArray(projectionsResult) ? projectionsResult : []);
      setArtifacts(artifactsResult);
    } catch (refreshError: any) {
      setError(refreshError?.message || 'Failed to refresh readiness data.');
    } finally {
      setBusy(null);
    }
  }, [
    getDataRoom,
    getFinancialProjections,
    getInfraStatus,
    getInvestorMetrics,
    getReadinessArtifacts,
    listPartners,
  ]);

  useEffect(() => {
    refresh();
  }, []);

  const handleRegister = async () => {
    if (!company.trim() || !deviceType.trim()) {
      setError('Company and device type are required.');
      return;
    }

    setBusy('register');
    setError(null);

    try {
      await registerPartner(company.trim(), deviceType.trim(), integrationLevel);
      setCompany('');
      setDeviceType('');
      setIntegrationLevel('basic');
      await refresh();
    } catch (registerError: any) {
      setError(registerError?.message || 'Failed to register partner.');
    } finally {
      setBusy(null);
    }
  };

  const handleCertify = async (partnerId: string) => {
    setBusy(partnerId);
    setError(null);
    try {
      await certifyPartner(partnerId);
      await refresh();
    } catch (certifyError: any) {
      setError(certifyError?.message || 'Failed to certify partner.');
    } finally {
      setBusy(null);
    }
  };

  const readyDocs = dataRoom.filter((doc) => doc.status === 'ready').length;
  const draftDocs = dataRoom.filter((doc) => doc.status === 'draft').length;
  const missingDocs = dataRoom.filter((doc) => doc.status === 'missing').length;
  const certifiedPartners = partners.filter((partner) => partner.certified).length;

  return (
    <div className="p-6 space-y-6 max-w-7xl">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-2">
          <div className="inline-flex items-center gap-2 rounded-full border border-[#1A1E26] bg-[#0D1117] px-3 py-1 text-[11px] uppercase tracking-[0.28em] text-[#8FA3B8]">
            <Rocket size={12} className="text-[#00E5E5]" />
            D17-D20
          </div>
          <div>
            <h1 className="text-2xl font-semibold tracking-tight text-[#E6EDF3]">Readiness and market proof</h1>
            <p className="max-w-3xl text-sm leading-6 text-[#8FA3B8]">
              Partner enablement, infra posture, investor-facing metrics, demo tracks, and the audit trail for
              pushing AgentOS from internal platform to external category narrative.
            </p>
          </div>
        </div>
        <Button variant="secondary" onClick={refresh} loading={busy === 'refresh'}>
          Refresh
        </Button>
      </div>

      {error && (
        <div className="rounded-lg border border-[#E74C3C]/30 bg-[#E74C3C]/10 px-4 py-3 text-sm text-[#F5B7B1]">
          {error}
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-4">
        <MetricCard
          icon={<BadgeCheck size={18} className="text-[#2ECC71]" />}
          label="Certified partners"
          value={`${certifiedPartners}/${partners.length}`}
          detail="Partner enablement baseline"
        />
        <MetricCard
          icon={<LineChart size={18} className="text-[#00E5E5]" />}
          label="ARR"
          value={metrics ? `$${metrics.arr.toLocaleString()}` : 'n/a'}
          detail={metrics?.modeled ? 'Modeled from local runtime usage' : 'Investor metrics endpoint'}
        />
        <MetricCard
          icon={<Globe2 size={18} className="text-[#00E5E5]" />}
          label="Infra"
          value={infra?.global_status || 'unknown'}
          detail={infra?.source_note || 'No infra data'}
        />
        <MetricCard
          icon={<FolderOpen size={18} className="text-[#F39C12]" />}
          label="Data room"
          value={`${readyDocs} ready`}
          detail={`${draftDocs} draft, ${missingDocs} missing - repo-backed index`}
        />
      </div>

      <div className="grid gap-4 xl:grid-cols-[1.05fr_0.95fr]">
        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Partner enablement</h3>
                <p className="mt-1 text-xs text-[#5F7389]">Register, certify, and track OEM or hardware integrations.</p>
              </div>
              <Building2 size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="grid gap-4 lg:grid-cols-[0.9fr_1.1fr]">
            <div className="space-y-3">
              <input
                value={company}
                onChange={(event) => setCompany(event.target.value)}
                placeholder="Partner company"
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] outline-none placeholder:text-[#4C6075]"
              />
              <input
                value={deviceType}
                onChange={(event) => setDeviceType(event.target.value)}
                placeholder="Device type"
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] outline-none placeholder:text-[#4C6075]"
              />
              <select
                value={integrationLevel}
                onChange={(event) => setIntegrationLevel(event.target.value as 'basic' | 'premium' | 'exclusive')}
                className="w-full rounded-lg border border-[#1A1E26] bg-[#0A0E14] px-3 py-2 text-sm text-[#E6EDF3] outline-none"
              >
                <option value="basic">basic</option>
                <option value="premium">premium</option>
                <option value="exclusive">exclusive</option>
              </select>
              <Button onClick={handleRegister} loading={busy === 'register'}>
                Register partner
              </Button>
            </div>

            <div className="space-y-2">
              {partners.length === 0 ? (
                <EmptyState
                  icon={Building2}
                  title="No partners yet"
                  description="Use the form to seed the partner registry and then certify integrations one by one."
                />
              ) : (
                partners.map((partner) => (
                  <div key={partner.id} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                    <div className="flex items-start justify-between gap-4">
                      <div>
                        <p className="text-sm font-medium text-[#E6EDF3]">{partner.company}</p>
                        <p className="mt-1 text-xs text-[#5F7389]">
                          {partner.device_type} · {partner.integration_level}
                        </p>
                        <p className="mt-2 text-xs text-[#5F7389]">{partner.registered_at}</p>
                        {partner.certified_at && (
                          <p className="mt-1 text-xs text-[#5F7389]">Certified at {partner.certified_at}</p>
                        )}
                        {partner.certification_note && (
                          <p className="mt-2 text-xs text-[#5F7389]">{partner.certification_note}</p>
                        )}
                      </div>
                      <div className="space-y-2 text-right">
                        <span
                          className={`inline-flex rounded-full px-2 py-1 text-[11px] uppercase tracking-[0.16em] ${
                            partner.certified
                              ? 'bg-[#2ECC71]/10 text-[#2ECC71]'
                              : 'bg-[#F39C12]/10 text-[#F39C12]'
                          }`}
                        >
                          {partner.certified ? 'Certified' : 'Pending'}
                        </span>
                        {!partner.certified && (
                          <div>
                            <Button
                              size="sm"
                              variant="secondary"
                              onClick={() => handleCertify(partner.id)}
                              loading={busy === partner.id}
                            >
                              Certify
                            </Button>
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </Card>

        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Market readiness evidence</h3>
                <p className="mt-1 text-xs text-[#5F7389]">Repo-backed demo tracks and documentary readiness artifacts.</p>
              </div>
              <ShieldEllipsis size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="space-y-4">
            <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
              <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">Demo tracks (repo-backed)</p>
              <div className="mt-3 space-y-2">
                {(artifacts?.demo_tracks || []).map((track) => (
                  <p key={track} className="text-sm leading-6 text-[#C5D0DC]">
                    {track}
                  </p>
                ))}
              </div>
            </div>

            <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
              <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">Evidence docs (repo-backed)</p>
              <div className="mt-3 space-y-2">
                {(artifacts?.evidence_docs || []).map((doc) => (
                  <div key={doc.path} className="flex items-center justify-between gap-3 rounded-lg border border-[#1A1E26] px-3 py-2">
                    <span className="text-sm text-[#C5D0DC]">{doc.path}</span>
                    <ArrowUpRight size={14} className="text-[#5F7389]" />
                  </div>
                ))}
              </div>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              {artifacts?.market_readiness && (
                <ArtifactBox artifact={artifacts.market_readiness} title="Market readiness" />
              )}
              {artifacts?.definitive_mode && (
                <ArtifactBox artifact={artifacts.definitive_mode} title="Definitive mode" />
              )}
            </div>
          </div>
        </Card>
      </div>

      <div className="grid gap-4 xl:grid-cols-[0.95fr_1.05fr]">
        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Infra and investor snapshot</h3>
                <p className="mt-1 text-xs text-[#5F7389]">Live probe snapshot plus explicitly modeled investor estimates.</p>
              </div>
              <Globe2 size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <MetricBox label="Growth" value={metrics ? `${metrics.mrr_growth_pct}%` : 'n/a'} detail={metrics?.source_note || 'No investor metrics source'} />
              <MetricBox label="Runway" value={metrics ? `${metrics.runway_months} mo` : 'n/a'} detail={metrics?.modeled ? 'Modeled estimate, not finance ledger.' : 'Investor metrics endpoint'} />
            </div>

            {infra?.probe_mode && (
              <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4 text-sm text-[#8FA3B8]">
                {infra.probe_mode} - {infra.source_note}
              </div>
            )}

            <div className="space-y-2">
              {(infra?.regions || []).map((region) => (
                <div key={region.region} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                  <div className="flex items-center justify-between gap-4">
                    <div>
                      <p className="text-sm font-medium text-[#E6EDF3]">{region.region}</p>
                      <p className="mt-1 text-xs text-[#5F7389]">{region.last_checked}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-sm text-[#C5D0DC]">{region.latency_ms} ms</p>
                      <p className="text-xs text-[#5F7389]">{region.status} - {region.probe_type}</p>
                    </div>
                  </div>
                  {region.note && <p className="mt-2 text-xs text-[#5F7389]">{region.note}</p>}
                </div>
              ))}
            </div>
          </div>
        </Card>

        <Card
          header={
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-[#E6EDF3]">Data room and projections</h3>
                <p className="mt-1 text-xs text-[#5F7389]">Repo-backed data room index plus clearly modeled projections.</p>
              </div>
              <FolderOpen size={16} className="text-[#00E5E5]" />
            </div>
          }
        >
          <div className="space-y-3">
            {dataRoom.slice(0, 6).map((doc) => (
              <div key={doc.name} className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <p className="text-sm font-medium text-[#E6EDF3]">{doc.name}</p>
                    <p className="mt-1 text-xs text-[#5F7389]">{doc.category}</p>
                    <p className="mt-2 text-sm text-[#C5D0DC]">{doc.description}</p>
                    {doc.path && <p className="mt-2 text-xs text-[#5F7389]">{doc.path}</p>}
                  </div>
                  <span
                    className={`inline-flex rounded-full px-2 py-1 text-[11px] uppercase tracking-[0.16em] ${
                      doc.status === 'ready'
                        ? 'bg-[#2ECC71]/10 text-[#2ECC71]'
                        : doc.status === 'draft'
                          ? 'bg-[#F39C12]/10 text-[#F39C12]'
                          : 'bg-[#E74C3C]/10 text-[#E74C3C]'
                    }`}
                  >
                    {doc.status}
                  </span>
                </div>
              </div>
            ))}

            {projections.length > 0 && (
              <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
                <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">Three-year projection (modeled)</p>
                <div className="mt-3 space-y-2">
                  {projections.map((projection) => (
                    <div key={projection.year} className="grid grid-cols-4 gap-2 text-sm text-[#C5D0DC]">
                      <span>{projection.year}</span>
                      <span>${Math.round(projection.arr).toLocaleString()}</span>
                      <span>{projection.users.toLocaleString()} users</span>
                      <span>${Math.round(projection.costs).toLocaleString()} costs</span>
                    </div>
                  ))}
                </div>
                {projections[0]?.modeled_note && (
                  <p className="mt-3 text-xs text-[#5F7389]">{projections[0].modeled_note}</p>
                )}
              </div>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
}

function MetricCard({
  icon,
  label,
  value,
  detail,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <div className="rounded-xl border border-[#1A1E26] bg-[#0D1117] p-4 shadow-md shadow-black/20">
      <div className="flex items-center justify-between">
        <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-[#0A0E14]">
          {icon}
        </span>
      </div>
      <p className="mt-4 text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">{label}</p>
      <p className="mt-2 text-2xl font-semibold text-[#E6EDF3]">{value}</p>
      <p className="mt-1 text-xs text-[#5F7389]">{detail}</p>
    </div>
  );
}

function MetricBox({ label, value, detail }: { label: string; value: string; detail: string }) {
  return (
    <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
      <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">{label}</p>
      <p className="mt-2 text-2xl font-semibold text-[#E6EDF3]">{value}</p>
      <p className="mt-1 text-xs text-[#5F7389]">{detail}</p>
    </div>
  );
}

function ArtifactBox({ artifact, title }: { artifact: RepoArtifact; title: string }) {
  return (
    <div className="rounded-xl border border-[#1A1E26] bg-[#0A0E14] p-4">
      <p className="text-[11px] uppercase tracking-[0.22em] text-[#5F7389]">{title}</p>
      <p className="mt-2 text-sm font-medium text-[#E6EDF3]">{artifact.path}</p>
      <p className="mt-1 text-xs text-[#5F7389]">{artifact.status}</p>
      {artifact.last_modified && <p className="mt-1 text-xs text-[#5F7389]">{artifact.last_modified}</p>}
    </div>
  );
}
