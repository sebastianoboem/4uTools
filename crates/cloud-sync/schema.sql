-- Eseguire una volta nel progetto Supabase.
-- Nel pannello: Authentication → abilita «Anonymous sign-ins».

create table uninstall_events (
  id uuid primary key,
  owner uuid not null default auth.uid(),
  package_name text not null,
  phone_hash text not null,
  uninstalled_at timestamptz not null,
  success boolean not null,
  notes text
);

create table flag_votes (
  owner uuid not null default auth.uid(),
  package_name text not null,
  phone_hash text not null,
  flag text not null check (flag in ('system', 'trusted', 'suspicious', 'none')),
  voted_at timestamptz not null,
  primary key (owner, package_name, phone_hash)
);

alter table uninstall_events enable row level security;
alter table flag_votes enable row level security;

-- owner viene sempre da auth.uid(), mai dal client.
create function push_uninstalls(rows jsonb) returns void
  language sql security definer
  set search_path = public
as $$
  insert into uninstall_events (id, owner, package_name, phone_hash, uninstalled_at, success, notes)
  select
    (x->>'id')::uuid,
    auth.uid(),
    x->>'package_name',
    x->>'phone_hash',
    (x->>'uninstalled_at')::timestamptz,
    (x->>'success')::boolean,
    x->>'notes'
  from jsonb_array_elements(rows) x
  where auth.uid() is not null
  on conflict (id) do nothing
$$;

create function push_votes(rows jsonb) returns void
  language sql security definer
  set search_path = public
as $$
  insert into flag_votes (owner, package_name, phone_hash, flag, voted_at)
  select
    auth.uid(),
    x->>'package_name',
    x->>'phone_hash',
    x->>'flag',
    (x->>'voted_at')::timestamptz
  from jsonb_array_elements(rows) x
  where auth.uid() is not null
  on conflict (owner, package_name, phone_hash) do update
    set flag = excluded.flag, voted_at = excluded.voted_at
    where flag_votes.voted_at < excluded.voted_at
$$;

revoke execute on function push_uninstalls(jsonb), push_votes(jsonb) from public, anon;
grant execute on function push_uninstalls(jsonb), push_votes(jsonb) to authenticated;

create schema private;
revoke all on schema private from public, anon, authenticated;

create function private.refresh_package_stats(p_package_name text) returns void
  language plpgsql security definer set search_path = public
as $$
begin
  insert into public.package_stats (
    package_name, uninstall_count, system_votes, trusted_votes, suspicious_votes
  )
  select p.package_name,
    coalesce(u.n, 0),
    count(distinct v.phone_hash) filter (where v.flag = 'system'),
    count(distinct v.phone_hash) filter (where v.flag = 'trusted'),
    count(distinct v.phone_hash) filter (where v.flag = 'suspicious')
  from (
    select package_name from flag_votes where package_name = p_package_name
    union
    select package_name from uninstall_events where package_name = p_package_name
  ) p
  left join flag_votes v on v.package_name = p.package_name
  left join (
    select package_name, count(*) n
    from uninstall_events
    where success and package_name = p_package_name
    group by 1
  ) u on u.package_name = p.package_name
  group by p.package_name, u.n
  on conflict (package_name) do update set
    uninstall_count = excluded.uninstall_count,
    system_votes = excluded.system_votes,
    trusted_votes = excluded.trusted_votes,
    suspicious_votes = excluded.suspicious_votes;
end $$;

revoke all on function private.refresh_package_stats(text) from public, anon, authenticated;

create table package_stats (
  package_name text primary key,
  uninstall_count bigint not null,
  system_votes bigint not null,
  trusted_votes bigint not null,
  suspicious_votes bigint not null
);
alter table package_stats enable row level security;
create policy package_stats_read on package_stats
  for select to authenticated using (true);
revoke all on table package_stats from public, anon, authenticated;
grant select on table package_stats to authenticated;
revoke select on table flag_votes, uninstall_events from anon, authenticated;

-- I client si iscrivono solo a questa tabella. Poi rileggono package_stats.
create table package_signals (
  package_name text primary key,
  changed_at timestamptz not null default now()
);
alter table package_signals enable row level security;
create policy package_signals_read on package_signals
  for select to authenticated using (true);
grant select on package_signals to authenticated;

create function touch_package_signal() returns trigger
  language plpgsql security definer set search_path = public
as $$
begin
  perform private.refresh_package_stats(new.package_name);
  insert into package_signals (package_name) values (new.package_name)
  on conflict (package_name) do update set changed_at = now();
  return new;
end $$;

create trigger flag_votes_signal
  after insert or update on flag_votes
  for each row execute function touch_package_signal();
create trigger uninstall_events_signal
  after insert on uninstall_events
  for each row execute function touch_package_signal();

alter publication supabase_realtime add table package_signals;
