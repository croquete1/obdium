# OBDium — Roadmap de desenvolvimento e auditoria técnica

**Baseline:** 2026-11-25 · **Repositório:** `croquete1/obdium` · **Licença:** GPL-3.0  
**Upstream:** https://github.com/provrb/obdium · **SHA auditado:** `fdaef0a3bdc293b1621e1b2f96b482fba7e4ee69`  
**Estado:** planeado; nenhum item de implementação está concluído por efeito deste documento.  
**Objetivo:** alternativa open-source ao OBD Auto Doctor Premium, começando por diagnóstico OBD-II/EOBD normalizado fiável e evoluindo para funções de oficina. Não implica equivalência universal a equipamentos de diagnóstico OEM.

## 1. Princípios e limites

- Manter Rust, Tauri 2, SQLite, suporte Windows/macOS/Linux, VIN offline, PIDs personalizados, modo demo e licença GPL-3.0. Preservar autoria e histórico do upstream; cumprir licenças de dependências e dados distribuídos.
- Melhorar incrementalmente, sem reescrita total. TypeScript/Vite e React são propostas a validar por decisão arquitetural, não pré-requisitos para corrigir o motor.
- Offline por defeito; sem conta, telemetria obrigatória, IA obrigatória ou subscrição. Dados pertencem ao utilizador. Sem ativação de serviços pagos sem autorização explícita.
- Separar capacidades OBD genéricas de funções OEM/UDS. Disponibilidade depende do veículo, ECU, protocolo, adaptador e plataforma. Nunca apresentar um PID/ECU como suportado sem evidência.
- Operações de leitura por defeito. Escritas e ativações exigem confirmação explícita, precondições documentadas, política de autorização no backend, exclusão mútua, auditoria e testes físicos. Não implementar manipulação de odómetros, contorno de imobilizadores/gateways, desativação de sistemas de emissões ou programação indiscriminada.
- Uma falha, timeout ou ausência de dados nunca é equivalente a zero ou a uma aprovação. DTC não prova que uma peça está avariada. Alertas não substituem avaliação técnica.
- Dados OEM e normas: obter informação de fontes legítimas/licenciadas; não copiar bases, código, textos ou interface proprietários do produto de referência.

## 2. Baseline verificada

- Última release identificada: [v1.5.1](https://github.com/provrb/obdium/releases/tag/v1.5.1), publicada em 09/06/2026; ativos macOS atualizados em setembro segundo [autor](https://github.com/provrb/obdium/issues/37#issuecomment-5622965294).
- Último commit main consultado: [10/09/2026](https://github.com/provrb/obdium/commit/fdaef0a3bdc293b1621e1b2f96b482fba7e4ee69). Último comentário do autor entre os consultados: [10/10/2026](https://github.com/provrb/obdium/issues/41#issuecomment-6103409841). O projeto não estava arquivado; atividade recente sobretudo da comunidade. Não assumir abandono definitivo.
- Três últimas execuções observadas falharam: [run 34507299894](https://github.com/provrb/obdium/actions/runs/34507299894), [run 34505664605](https://github.com/provrb/obdium/actions/runs/34505664605), [run 34505066945](https://github.com/provrb/obdium/actions/runs/34505066945). Causas exatas dos jobs ainda por diagnosticar.
- Estrutura atual: `backend/src` (motor, PID, bridge, replay, VIN), `frontend/` (HTML/CSS/JS), `.github/workflows/publish.yml`, bases SQLite e dados demo. Sem workflow dedicado de PR nem diretório de testes identificado na árvore auditada; inventariar testes inline antes de concluir que não existem.
- PRs upstream [#42](https://github.com/provrb/obdium/pull/42) e [#43](https://github.com/provrb/obdium/pull/43) abertas no momento da auditoria. O autor da PR #42 relata testes BMW F30/OBDLink EX; não são testes independentes nossos. Rever patches efetivos (não apenas descrições), preservar autoria e adicionar regressões. Não integrar automaticamente comandos UDS de escrita.

## 3. Comparativo de funcionalidades

Legenda: **E** = implementação identificada/documentada, sem certificação física; **P** = parcial, defeituosa ou cobertura por validar; **N** = não identificada na auditoria; **?** = informação insuficiente. Auto Doctor: **F/P** = Free e Premium; **P** = Premium; **L/P** = Free limitado e Premium; **?** = não confirmado pelas fontes. Funcionalidades podem variar por plataforma/veículo. IDs Fxx remetem para as fases abaixo.

| Funcionalidade | OBDium atual | OBD Auto Doctor | Fase |
|---|---|---|---|
| DTC confirmados (03) | P | F/P | F04 |
| DTC pendentes (07) | N | F/P | F04 |
| DTC permanentes (0A) | P | F/P | F04 |
| Apagar DTC/MIL (04) | P | P | F04 |
| Freeze Frame (02) | P | P | F04 |
| Partilha/exportação de diagnóstico | E (JSON) | F/P | F08 |
| Pesquisa de DTC | P (base de descrições; pesquisa global a verificar) | F/P | F04 |
| DTC específicos de fabricante | P (cobertura por validar) | P | F10 |
| Readiness desde reset | P | F/P | F05 |
| Readiness ciclo atual | N | F/P | F05 |
| Testes de oxigénio (05, quando aplicável) | N | P | F05 |
| Resultados monitores internos (06) | N (stubs) | P | F05 |
| Ativação de serviços (08) | N | P | F11 |
| Contadores IUPR (09) | N | P | F05 |
| Sensores em tempo real (01) | E/P (cobertura a medir) | L/P | F03/F05 |
| Gráficos em tempo real | E | L/P | F07/F08 |
| Grelha de gráficos | P | P | F08 |
| Histogramas | N | P | F08 |
| Exportar gráficos como imagem | N | P | F08 |
| Registar sensores com timestamps e CSV | N | P | F08 |
| Protocolo e requisitos OBD | E/P | F/P | F05 |
| Nome/descrição por ECU | P | F/P | F03/F05 |
| VIN (09) | P (parsing a corrigir) | P | F05 |
| CALID (09) | N | P | F05 |
| CVN (09) | N | P | F05 |
| Número de série de motor | N | P | F05 |
| Homologação/norma de emissões | N | P | F05 |
| Múltiplas ECUs OBD normalizadas | P | F/P | F03 |
| Gasolina/diesel | E/P | F/P | F05 |
| Unidades métricas/imperiais | E | F/P | F07 |
| Consola técnica | E (restringir escritas) | P | F06/F11 |
| Log de dados/comunicação | E (replay JSON) | P | F08 |
| USB/serial | E/P | Sim | F06 |
| Bluetooth Classic via COM virtual | P (problemas reportados) | Sim | F06 |
| Bluetooth Classic nativo | N | Sim, conforme SO | F06 |
| BLE | N | Sim, conforme SO/adaptador | F06 |
| Wi-Fi/TCP | N | Sim | F06 |
| Windows | E/P | Sim | F01/F12 |
| macOS Intel/ARM | E/P | Sim | F01/F12 |
| Linux | E/P | Histórico; não listado nos requisitos atuais | F01/F12 |
| Android/iOS | N | Sim (funcionalidades variam) | F13 |
| PT-PT e internacionalização | N | Idiomas por confirmar | F07 |
| VIN detalhado offline (vPIC) | E/P | ? | F09 |
| PIDs personalizados/Mode 22 | E/P | ? | F10 |
| Reprodução offline determinística | P (aleatória) | ? | F02/F08 |
| Código aberto | GPL-3.0 | Comercial/proprietário | Todas |
| Histórico por veículo e relatórios PDF de oficina | N | ? | F09 |
| Comparação de sessões antes/depois | N | ? | F09 |
| Alertas, estatísticas e perfis de painel | N/P | ? | F08 |
| API/CLI e plugins | N | ? | F12 |

Fontes do comparativo consultadas em 25/11/2026: [planos atuais](https://www.obdautodoctor.com/pricing/), [funcionalidades](https://www.obdautodoctor.com/features/), [parâmetros suportados](https://www.obdautodoctor.com/help/articles/supported-obd-parameters/), [protocolos](https://www.obdautodoctor.com/help/articles/supported-obd-protocols/), [adaptadores](https://www.obdautodoctor.com/help/articles/supported-obd-adapters/), [requisitos de sistema](https://www.obdautodoctor.com/help/articles/system-requirements/), [histórico Linux](https://www.obdautodoctor.com/release-notes/linux/), [contadores IUPR](https://www.obdautodoctor.com/tutorials/understanding-in-use-performance-tracking-counters/). O produto documenta mais de 126 parâmetros powertrain (não um objetivo de contagem cega), até seis séries sobrepostas e até quatro gráficos independentes; validar cobertura, qualidade e desempenho. A coluna '?' não equivale a ausência de funcionalidade.

## 4. Registo de erros e riscos

Todos abertos. **C** = confirmado por inspeção do código; **R** = risco a reproduzir; **U** = reportado por upstream/comunidade. P0 bloqueia release fiável, P1 importante, P2 evolução. Links relativos ao SHA baseline, não à main mutável. Cada item exige teste que falha antes da correção e passa depois, quando aplicável.

| ID | Prioridade/evidência | Problema e localização | Correção e aceitação |
|---|---|---|---|
| BUG-001 | P0 C/U | [`obd.rs:655-701`](https://github.com/provrb/obdium/blob/fdaef0a3bdc293b1621e1b2f96b482fba7e4ee69/backend/src/obd.rs#L655-L701): limpeza de RX após envio descarta resposta rápida (#42). | Limpar apenas antes de nova transação quando adequado; teste fake transport com resposta imediata, tardia e parcial. |
| BUG-002 | P0 C/U | [`diagnostics.rs:424-529`](https://github.com/provrb/obdium/blob/fdaef0a3bdc293b1621e1b2f96b482fba7e4ee69/backend/src/pid/diagnostics.rs#L424-L529): split textual de 43/4A, trimming e concatenação incorretos. P143C reportado como C0A01 (#43). | Parser binário por protocolo/ECU/serviço, contagem e padding validados; fixtures com 43 no payload, vários DTC/ECU, zeros legítimos e frames incompletos. |
| BUG-003 | P0 C | `bridge/events.rs:339-363` só consulta 03/0A; `frontend/scripts/events.js:281-329` chama aos restantes pendentes. | Tipos Confirmed/Pending/Permanent; consultar 07 separadamente, preservar status e ECU; teste com mesmo DTC nos três estados. |
| BUG-004 | P0 C | `frontend/scripts/features.js:15-33` anuncia limpeza antes da resposta; `bridge/events.rs:365-371` ignora resultado. | Confirmação explícita, aguardar ACK válido por ECU; falha/timeout/parcial visíveis, reler; nenhum falso sucesso ou limpeza de UI antecipada. |
| BUG-005 | P0 C/R | `response.rs` descarta respostas subsequentes para extrair payload e agrega informação ECU. | Frames estruturados por endereço, serviço, PID, timestamp e qualidade; preservar ECU em cada amostra e DTC. |
| BUG-006 | P0 C | `response.rs:get_component` devolve 0 para byte ausente, panics em hex inválido. | Result/Option tipado; limites e erros explícitos; fuzzing de bytes, ausência != 0. |
| BUG-007 | P1 C | `mid.rs` contém vários `todo!()` em Mode 06. | Implementar leitura/decodificação ou devolver Unsupported sem panic; limites e escalas validados. |
| BUG-008 | P0 C | `tauri.conf.json`: CSP null e schema v1 com Tauri 2; HTML dinâmico em scripts. | CSP restritiva, schema correto, remover sinks perigosos/sanitizar dados e testes de injeção. |
| BUG-009 | P1 C/U | `obd.rs` acoplado a SerialPort; Bluetooth falha (#19/#39/#45), portas virtuais (#38). | Abstração Transport, deteção manual/automática, diagnóstico de permissões; testar USB, SPP, BLE e TCP. |
| BUG-010 | P0 R | `stats.rs` múltiplos loops e mutex partilhado, I/O síncrono em tarefas; contenção/cancelamento por validar. | Worker único por sessão com fila, prioridades, deadlines, backpressure; nenhuma starvation; testar stress/desligar. |
| BUG-011 | P0 C | `.github/workflows/publish.yml` actions antigas, upload/download v3 e publish dependente de release; CI observado vermelho. | Actions mantidas e SHA pinning, dependências Tauri 2 por SO, logs e builds verificados; determinar causa real antes de corrigir. |
| BUG-012 | P0 C | Sem workflow PR dedicado identificado na árvore. | PR/push CI fmt/clippy/test/build, frontend lint/test, security audit; não permitir merge de código sem validação. |
| BUG-013 | P1 C | README 1.5.1 vs Cargo/tauri.conf 1.5.0. | Fonte única e verificação de consistência, tag/manifestos sincronizados. |
| BUG-014 | P0 R | `bridge/events.rs:206-338`, `bridge/mod.rs`: listeners criados a cada ligação e só dois explicitamente removidos; risco de sessões antigas. | Lifecycle único, geração de sessão e cleanup RAII; 100 ciclos conectar/desconectar, sem callbacks duplicados nem comandos para sessão antiga. |
| BUG-015 | P0 C | `obd.rs:655-701` read_until interrompe em erro e pode retornar Ok parcial sem prompt. | Deadline global, tamanho máximo, distinguir timeout/desligado/mensagem truncada; testes de stream incompleto e read zero. |
| BUG-016 | P0 C/R | `obd.rs:813-846` redireciona 01 a 02; `response.rs:get_payload_components` aceita 41/62 mas não 42; frame index ausente no caminho observado. | Mode 02 explícito por ECU e frame, parsing 42 e PID 02 conforme protocolo; não confundir dados live e snapshot. |
| BUG-017 | P0 C | `backend/capabilities/migrated.json`: allow-write-file/remove com scope */**. | Privilégio mínimo, paths selecionados/autorizados e app-data, não permitir remoção arbitrária; testes negativos. |
| BUG-018 | P1 C | `replay.rs:save_request` lê, desserializa e reescreve JSON completo por pedido. | Writer em lote append-only/SQLite, quotas e flush, desempenho não quadrático. |
| BUG-019 | P1 C | `replay.rs` usa expect e converte JSON inválido em vetor vazio, podendo sobrescrever ficheiro. | Preservar original, escrita atómica, recuperação, erros tipados; testes disco cheio, permissões e JSON corrompido. |
| BUG-020 | P1 C | `replay.rs:get_recorded_response` escolhe resposta aleatória por comando. | Replay cronológico determinístico com seed opcional, tempo virtual e fixtures; manter demo aleatória como modo separado se útil. |
| BUG-021 | P2 C | `main.rs:try_load_vpic_database` exige ~1,45 GB descomprimidos. | Medir espaço e startup, carga opcional/lazy, informar requisitos, compressão/shards e migração sem perder dados. |
| BUG-022 | P1 C | `main.rs:try_load_vpic_database` valida tamanho, escreve direto e retorna Ok após falha de copy. | Hash/versão e verificação SQLite, espaço livre, ficheiro temporário + rename atómico, erro real e recuperação após interrupção. |
| BUG-023 | P0 C/R | `diagnostics.rs:408-422` ignora Mode 03 se PID 0101 indicar zero; próprio comentário alerta para >3 DTC. | Consultar por serviço/ECU sem atalho indevido, diferenciar zero e indisponível; regressão com múltiplas ECUs e >3 códigos. |
| BUG-024 | P0 U/R | #42: filtragem de ECU, inicialização STN, STOPPED e montagem ISO-TP/VIN. | Validar patches independentes, 11/29-bit CAN, sequência/length/timeout; comparar com captura conhecida e hardware. |
| BUG-025 | P1 C | `frontend/scripts/events.js:384-389` disponibilidade de monitor apresentada como COMPLETE/NOT COMPLETE. | Distinguir não suportado, incompleto, completo e desconhecido; readiness correto para gasolina/diesel. |
| BUG-026 | P1 R | `lib.rs`: caminhos relativos para bases DTC/Mode22; bundles resolvem recursos de forma distinta. | Resolver através de recursos Tauri/app-data; teste instalado com CWD arbitrário em três SO. |
| BUG-027 | P1 R | `obd.rs:get_vin` usa expect no parse de hex. | Validar mensagem e VIN, nunca panic com resposta externa; testar VIN inválido/parcial/múltiplas ECUs. |
| BUG-028 | P1 R | Base DTC e PIDs específicos podem não distinguir fabricante/motor; [#21](https://github.com/provrb/obdium/issues/21). | Resolver por contexto e proveniência; desconhecido se ambíguo, sem inventar descrição. |
| BUG-029 | P1 U | [#46](https://github.com/provrb/obdium/issues/46) Fedora AppImage; [#16](https://github.com/provrb/obdium/issues/16) incompatibilidade libgcrypt. | Reproduzir em distro suportada, corrigir bundling, validar instalação/arranque e documentar limites. |
| BUG-030 | P1 U | [#41](https://github.com/provrb/obdium/issues/41) documentação incompleta. | Manual e matriz de compatibilidade baseados em testes reais. |

## 5. Arquitetura-alvo (incremental)

```text
Desktop Tauri 2 (UI tipada, i18n, gráficos)
  -> API interna tipada (comandos/respostas/eventos com session_id)
     -> Session manager / scheduler / safety policy
        -> Protocol core: OBD-II, ELM/STN, ISO-TP, extensões UDS restritas
           -> Transport: Serial/USB, SPP, BLE, TCP, Mock/Replay
        -> Diagnostics: DTC, readiness, MID, VIN, PID registry
        -> Storage: SQLite, migrações, sessões, amostras, relatórios
        -> Rules: alertas explicáveis, sem diagnóstico inventado
```

Possível workspace Rust: `obd-core`, `obd-protocols`, `obd-transport`, `obd-diagnostics`, `obd-storage`, `obd-simulator`, `obd-reports`; `apps/desktop`, `packages/ui`, `packages/shared-types`, `tests/{fixtures,integration,hardware,regression}`. Não criar crates vazias sem benefício; extrair após contratos e testes. Modelos: `Session`, `AdapterCapabilities`, `EcuId`, `Request`, `Frame`, `DiagnosticResult<T>`, `Measurement{timestamp,ecu,pid,value,unit,quality}`, `Dtc{service,status,ecu,code,description,source}`, `Vehicle`, `DiagnosticSession`, `FreezeFrame`, `MonitorResult`. IDs estáveis, erros tipados e unidades canónicas.

## 6. Roadmap operacional

**Estados:** TODO / IN_PROGRESS / REVIEW / BLOCKED / DONE. Todos os itens abaixo começam TODO. P0 primeiro; P1 após motor fiável; P2 extensões. Um item só fecha com critérios de aceitação, PR e evidências. IDs estáveis para futuras issues; dependências entre fases indicadas.

### F01 — Fundação, CI/CD e segurança (P0)
Dependências: nenhuma. Erros 008,011–013,017,026,029.
- [ ] F01.01 Registar upstream SHA, fork completo, remotes, licença e estratégia de sincronização; não reescrever histórico.
- [ ] F01.02 Reproduzir build local e CI, diagnosticar os três runs falhados; corrigir dependências de sistema/GTK/WebKit e packaging com Tauri 2; toolchain Rust fixada e lockfile respeitado.
- [ ] F01.03 CI para PR e push: fmt, clippy, cargo test --locked, build release; matriz Windows x64/macOS Intel+ARM/Linux x64 (Fedora/Ubuntu instalação); checks de frontend e lint Markdown.
- [ ] F01.04 Atualizar actions suportadas, permissões mínimas, pinning por SHA, secrets protegidos; proteger main e evitar execuções de código não confiável com tokens de escrita.
- [ ] F01.05 Auditoria Rust/dependências e licenças (cargo audit/deny), dependabot ou equivalente, SBOM, checksums e pipeline de release com tags; sem assinatura fictícia.
- [ ] F01.06 Corrigir CSP, IPC/HTML e capability scopes; validação de paths, importações, parâmetros e comandos.
- [ ] F01.07 Unificar versão; recursos resolvidos com app-data, instalação limpa e upgrade; rollback documentado.
- [ ] F01.08 Configurar revisão CodeRabbit OSS apenas se gratuita e elegível; sem pagamento nem consumo adicional. Alternativa: revisão manual/CI. Não depender da ferramenta como prova de correção.
- [ ] F01.09 Criar templates PR/issues, CONTRIBUTING, política de segurança e ADRs, regras de publicação, matriz de compatibilidade.
**Aceitação:** CI verde nos SO alvo; instalação/arranque de release candidata verificados; testes negativos de IPC/filesystem; nenhuma despesa autorizada implicitamente; documentação e baseline reproduzíveis.

### F02 — Revisão upstream, testes e simulador (P0; após F01.01)
- [ ] F02.01 Rever patches reais #42/#43, divergências e risco de comandos de escrita; repartir alterações, preservar autoria, testes antes de integrar; não enviar PR upstream sem escopo claro.
- [ ] F02.02 Criar golden fixtures de ELM/STN, eco on/off, headers, prompt, NO DATA, STOPPED, erros, respostas rápidas/lentas, várias ECUs, 11/29-bit CAN e protocolos legados.
- [ ] F02.03 Mock Transport e relógio virtual; replay por timestamp e ordem, cenários reproduzíveis, seed opcional; testar disconnect, latência, perda de frames e corrupção.
- [ ] F02.04 Testes unitários, integração, property/fuzz tests e regressão; fixtures anonimizadas (VIN e identificadores removidos), sem partilha automática.
- [ ] F02.05 Testar regressões DTC P143C, multi-frame VIN, resposta imediata e confirmação de limpeza sem veículo real. Implementar runner hardware opt-in separado de CI.
**Aceitação:** testes determinísticos reproduzem os bugs; qualquer sequência de bytes inválida produz erro tipado, nunca panic; modo demo continua funcional.

### F03 — Motor de comunicação e sessão (P0; F01/F02)
- [ ] F03.01 Interface Transport com open/read/write/close, cancelamento, deadlines, limites de buffer e classificação de erros; preservar backend serial existente.
- [ ] F03.02 Corrigir RX clear e parsing de prompt; transações completas, timeout absoluto, retries limitados apenas para operações idempotentes; distinguir adaptador conectado de ECU comunicante.
- [ ] F03.03 Sessão única com estados disconnected/connecting/ready/degraded/disconnecting; cancelar filas e listeners, rejeitar eventos antigos por session_id; reconectar sem repetir escritas.
- [ ] F03.04 Scheduler único com prioridade, fairness, frequência por PID, batches quando suportados, backpressure e métricas de latência; não bloquear UI.
- [ ] F03.05 Negociação de protocolos, autodeteção/manual, inicialização ELM/STN e quirks de firmware; distinguir respostas 11-bit/29-bit, ISO 9141-2, ISO 14230-4, SAE J1850 e ISO 15765-4 conforme capacidade.
- [ ] F03.06 Parser binário com ISO-TP, sequência, length, timeout e ECU de origem; não misturar frames. Distinguir erros de transporte, resposta negativa, não suportado, indisponível e malformed.
- [ ] F03.07 Descobrir ECUs OBD normalizadas e PIDs suportados por ECU; cache por sessão, sem sondagem OEM indiscriminada.
- [ ] F03.08 Resolver valores ausentes/panics (BUG-005/006/010/014/015/024/027), testes de reconexão, stress e memória.
**Aceitação:** 100 ciclos de reconexão no simulador sem duplicação, nenhuma leitura incorreta de resposta rápida, isolamento das ECUs, filas limitadas e validação com USB ELM/STN antes de declarar suporte físico.

### F04 — DTC e Freeze Frame (P0; F03)
- [ ] F04.01 Decodificar 03/07/0A por serviço e ECU, preservar estados, timestamp, raw evidence; não usar contagem 0101 para omitir consulta.
- [ ] F04.02 Resolver descrições genéricas P/B/C/U, pesquisa local, separação de códigos específicos OEM com contexto, fonte/versão/idioma; não confundir código com diagnóstico conclusivo.
- [ ] F04.03 Mode 02 com frame index e ECU, DTC associado e dados originais; não substituir valores live por snapshots.
- [ ] F04.04 Mode 04: guardar diagnóstico e avisar perda de Freeze Frame/readiness; confirmação explícita, ACK por ECU, resultado parcial/erro, reler e nunca prometer apagar permanentes. Não repetir automaticamente.
- [ ] F04.05 UI de DTC com filtros, estado, descrição, ECU, contexto, exportação tipada e comparação; causas/sintomas e verificações apenas com fonte validada/licenciada.
**Aceitação:** fixtures de DTC com 43 interno, >3 códigos, vários módulos, duplicados e zeros válidos; Mode 07 distinto; Freeze Frame 42 validado; nenhuma mensagem de sucesso antes da confirmação; teste físico controlado para escrita.

### F05 — Paridade de diagnóstico normalizado (P1; F03/F04)
- [ ] F05.01 Inventariar PIDs SAE J1979/DA suportados e comparar com referência; metadados de serviço, bytes, unidades, scaling, limites e bitmaps; expandir catálogo sem duplicar. Gasolina/diesel e parâmetros adicionais conforme suporte.
- [ ] F05.02 Readiness desde reset e ciclo atual (01/01 e 01/41), diferenças spark/compression, MIL, distância/tempo desde reset, warmups; distinguir unsupported/incomplete/complete/unknown.
- [ ] F05.03 Mode 05 para protocolos aplicáveis; em CAN usar monitores pertinentes do Mode 06.
- [ ] F05.04 Mode 06: descoberta MID/TID, resultado/limites/unidade/scaling, sinal, testes de catalisador, misfire, sensores O2, EGR, EVAP, NOx, partículas etc. quando expostos; retirar stubs ou marcar explicitamente unsupported.
- [ ] F05.05 Mode 09: VIN, ECU name, CALID, CVN, IUPR, restantes itens normalizados disponíveis (serial do motor/homologação quando suportados), protocolo e norma OBD; montar ISO-TP antes de descodificar.
- [ ] F05.06 Página de emissões e inspeção (sem certificar aprovação legal): monitores, contexto DPF/EGR/SCR/NOx e valores suportados, distinção OEM vs genérico.
**Aceitação:** cada serviço tem fixtures positivas, negativas, não suportadas e truncadas; escalas validadas com especificações autorizadas e medições de referência; matriz de cobertura e comparação publicada.

### F06 — Adaptadores e transportes (P1; F03)
- [ ] F06.01 USB/serial Windows COM, Linux tty/udev e macOS, seleção manual, baud rate, portas virtuais, permissões e diagnóstico (#19/#38/#45).
- [ ] F06.02 Bluetooth Classic SPP nativo ou porta virtual, descoberta e emparelhamento orientado; BLE GATT com perfis específicos por adaptador e MTU/fragmentação.
- [ ] F06.03 TCP/Wi-Fi com host/porta configuráveis, timeout e reconexão; descoberta opcional limitada, comunicação apenas com dispositivo escolhido.
- [ ] F06.04 ELM327, STN/OBDLink EX/MX+/CX, Vgate/vLinker e clones testados individualmente; identificar firmware/capacidades sem prometer universalidade.
- [ ] F06.05 Assistente de ligação, diagnóstico de erros, teste de latência, log redigido, perfis guardados e matriz adaptador/OS/protocolo.
**Aceitação:** testes fake por transporte; ensaios físicos por método e SO antes de rotular como suportado; desligar adaptador/ignição não deixa tarefas ou UI bloqueadas.

### F07 — Interface e acessibilidade (P1; contratos F03/F04)
- [ ] F07.01 Auditoria UX e design system próprio, navegação Connection/Dashboard/DTC/Readiness/Monitors/ECU/VIN/History/Reports/Settings; manter identidade e ações claras.
- [ ] F07.02 Migrar JS incrementalmente para TypeScript e Vite; decidir React ou alternativa por ADR, sem reescrever tudo. IPC tipado e testes de contrato.
- [ ] F07.03 Layout responsivo desktop, temas claro/escuro, teclado, foco visível, labels e contrastes; reduzir largura mínima atual; sem cor como único indicador de estado.
- [ ] F07.04 i18n PT-PT e inglês primeiro, unidades métricas/imperiais por categoria, formatos de data/número locais; outras traduções comunitárias.
- [ ] F07.05 Dashboard editável: cards, indicadores digitais/analógicos, seleção de PID por ECU, perfis guardados, full screen; distinguir qualidade e dados desatualizados.
- [ ] F07.06 Testes E2E em simulador: ligar, navegar, desconectar, DTC/readiness/exportação, acessibilidade e screenshots.
**Aceitação:** fluxos críticos funcionam sem rato, sem perda de funcionalidades existentes, 1024x768 alvo a validar, dados simulados claramente rotulados; revisão visual e testes E2E.

### F08 — Monitorização, gravações e análise (P1; F03/F07)
- [ ] F08.01 Gráficos em tempo real com pelo menos 6 séries sobrepostas e 4 painéis independentes como alvo de paridade; seleção por ECU, unidades/eixos, zoom, pausa e cursores.
- [ ] F08.02 Histogramas e mínimo/média/máximo, percentis opcionais; amostras inválidas não entram em estatísticas, timestamp e intervalos claros.
- [ ] F08.03 Frequência configurável por sensor, downsampling para visualização sem alterar dados brutos, buffer limitado e sessões longas; medir polling/latência por adaptador.
- [ ] F08.04 Logging persistente append-only/SQLite com lote, timestamp e qualidade; recuperação após falha, quotas e rotação; importar requests.json legado sem destruir original.
- [ ] F08.05 Exportar CSV de sensores, JSON estruturado, TXT log e PNG de gráficos, com unidades, ECU, timestamps e metadados; validar escaping/CSV injection e exportações grandes.
- [ ] F08.06 Replay cronológico, velocidade/seek, comparação de séries, modo simulado sem transporte físico; anonimização antes de partilhar.
- [ ] F08.07 Alertas configuráveis com histerese/debounce, regras explicáveis e contexto; estatísticas de viagens e consumo estimado apenas quando entradas válidas; indicar incerteza.
**Aceitação:** sessão longa (alvo 2h) sem crescimento de memória ilimitado; dados exportados correspondem às amostras originais; sessões corrompidas preservadas; demo reproduzível; benchmark por hardware publicado.

### F09 — Veículos, armazenamento e modo oficina (P1; F04/F08)
- [ ] F09.01 SQLite com migrações transacionais, índices e esquema Vehicle/Session/ECU/DTC/Sample/Report; backup/restore e exclusão/exportação de dados pelo utilizador.
- [ ] F09.02 Garagem local: VIN, marca/modelo/motor/ano, matrícula opcional, quilometragem manual ou suportada, notas e perfis; distinguir valores inferidos/manuais/ECU.
- [ ] F09.03 VIN offline vPIC: validar formato conforme região, cobertura EUA/Europa explicitada, fonte/versão, importação segura e lazy loading; hash, espaço, integridade e instalação atómica. Não impor dígito de controlo norte-americano a todos os VIN europeus.
- [ ] F09.04 Histórico por veículo com sessões, avarias, snapshots e observações técnicas; comparar antes/depois por ECU e timestamp sem confundir mudança de contexto.
- [ ] F09.05 Relatórios PDF próprios, personalizáveis com logótipo/dados da oficina, identificação, escopo do scan, DTC, readiness, sensores, gráficos, limitações, data e técnico; PDF legível em várias páginas, impressão A4, CSV/JSON complementares.
- [ ] F09.06 Modo oficina multi-veículo, pesquisa, filtros, arquivo e perfis; sem dados pessoais obrigatórios, retenção configurável.
**Aceitação:** criar/editar/arquivar veículo, duas sessões comparáveis, backup/restore íntegro, relatório multipágina validado e exportações sem envio externo automático.

### F10 — OEM, PIDs personalizados, diesel, híbridos/EV (P2; F03/F05)
- [ ] F10.01 Registry declarativo de PIDs e DIDs com fórmula segura limitada, tipos, endian, bytes, unidades e proveniência; validar expressão/limites, rejeitar comandos de escrita importados.
- [ ] F10.02 Importar/exportar perfis e biblioteca comunitária versionada, revisão, licenças, assinaturas/hashes e matriz de compatibilidade; sem distribuir informação proprietária não autorizada.
- [ ] F10.03 Suporte Mode 22/UDS de leitura por perfil documentado, endereçamento e descoberta permitidos; separar OBD genérico de diagnóstico OEM. Não assumir que 7E0–7E3 identificam todos os módulos.
- [ ] F10.04 Módulos adicionais (TCM, ABS, SRS, BCM, EPS, HVAC, BMS, TPMS, gateway) apenas quando documentação legítima, hardware e testes comprovam suporte; DPF é sistema, não necessariamente ECU autónoma.
- [ ] F10.05 Perfis diesel (DPF pressão diferencial/carga/estado de regeneração, EGR, SCR/AdBlue, NOx, turbo, combustível, EGT) e híbridos/EV (SoC, temperaturas, tensões e SoH se disponibilizado); rotular estimativas e limites.
- [ ] F10.06 Investigar OBDonUDS/J1979-2 e WWH-OBD/ISO 27145 como projeto separado e dependente de documentação/testes. Não anunciar cobertura universal.
**Aceitação:** pelo menos um perfil por família escolhida com fixtures licenciadas, veículo/adaptador validados, read-only por defeito, fallback unsupported e documentação do que não funciona.

### F11 — Serviços ativos e segurança operacional (P2; F04/F05/F10)
- [ ] F11.01 Mode 08: catálogo allowlist de operações normalizadas suportadas, confirmação específica e precondições documentadas; separar comando e resultado.
- [ ] F11.02 Modelo de risco, permissões, bloqueio de polling concorrente, sessão explícita, timeouts/cancelamento e auditoria por ECU; nunca repetir operação ativa sem consentimento.
- [ ] F11.03 Avaliar operações OEM apenas individualmente com documentação legal, ambiente controlado, avaliação de segurança e validação física. Regeneração DPF, adaptações e outras rotinas potencialmente perigosas ficam bloqueadas até avaliação específica; sem promessas de suporte.
- [ ] F11.04 Consola técnica: leitura permitida, validação e aviso de operações desconhecidas, escritas desativadas por defeito; nenhum bypass de política pelo terminal/IPC/plugin.
**Aceitação:** operação não autorizada/condição desconhecida é recusada, nenhuma execução em demo ou reconexão, auditoria preservada, teste físico aprovado para cada operação. Não declarar equivalência profissional sem validação.

### F12 — Ecossistema, documentação e lançamento (P1/P2; F01–F11 conforme escopo)
- [ ] F12.01 CLI read-only com saída JSON/CSV e simulação; API local opt-in autenticada e limitada se houver caso de uso. Plugins declarativos isolados sem acesso arbitrário a hardware/filesystem; ADR antes de SDK.
- [ ] F12.02 Manual PT-PT/EN, instalação, adaptadores, resolução de problemas, segurança e compatibilidade; retomar documentação upstream #41; contribuição comunitária.
- [ ] F12.03 Instaladores Windows MSI/NSIS, macOS DMG (Intel/ARM), Linux AppImage/deb/rpm conforme suporte, checksums/SBOM/licenças, estratégia de atualização e rollback; código assinado apenas se infraestrutura existir.
- [ ] F12.04 Matriz HIL: USB ELM, STN, SPP, BLE, TCP, gasolina/diesel, CAN 11/29 bit e legados disponíveis; registar protocolo, firmware, veículo anonimizado, build e limitações; não testar escritas em circulação.
- [ ] F12.05 Release gates: regressões P0 fechadas, CI verde, testes físicos mínimos e notas de release; beta antes de estável. Contribuir upstream apenas mudanças revisadas/compatíveis, mantendo fork independente.
- [ ] F12.06 Preservação de projeto: mirror Git, cópia de releases/docs/issues quando necessário, plano de sincronização upstream, atribuição e atualização da matriz de paridade.
**Aceitação:** releases reproduzíveis/documentadas em SO suportados, nenhuma P0 aberta no escopo, relatórios de CI/hardware e auditoria; não anunciar suporte onde o hardware ainda é bloqueador.

### F13 — Aplicações móveis (P2 opcional; após desktop estável)
- [ ] F13.01 ADR Tauri 2 mobile vs alternativa, reutilização do core Rust, permissões Android/iOS, limitações Bluetooth Classic iOS, BLE/Wi-Fi, background e bateria.
- [ ] F13.02 UI touch, dashboards, DTC, histórico e exportação local; paridade com desktop apenas quando viável.
- [ ] F13.03 Validação física, requisitos de lojas, assinatura e distribuição; não contratar contas pagas nem publicar automaticamente sem aprovação.
**Aceitação:** apps testadas em dispositivos reais e transportes suportados, limitações explícitas. Não bloquear objetivo de paridade desktop por mobile.

## 7. Entregas e gates

| Marco | Escopo | Condição de aprovação |
|---|---|---|
| M0 baseline | F01/F02 | Build e CI reproduzíveis, fixtures e segurança inicial |
| M1 diagnóstico fiável | F03/F04 | P0 do motor/DTC fechadas, regressões e testes físicos mínimos |
| M2 paridade desktop | F05–F08 | Comparativo item a item com cobertura demonstrada, transportes e UI validados; Mode 08 pendente se ainda bloqueado |
| M3 modo oficina | F09 | Histórico, PDF, backup, sessões e privacidade |
| M4 extensões | F10/F11/F12 | Perfis OEM e operações ativas aprovados individualmente; releases verificadas |
| M5 mobile opcional | F13 | Compatibilidade por plataforma demonstrada |

Paridade Premium integral só pode ser declarada quando todas as linhas de referência aplicáveis estiverem implementadas, incluindo Mode 08 com segurança, ou quando as exclusões forem declaradas. Não confundir contagem de funcionalidades com equivalência técnica. Sem prazos ou percentagens inventados.

## 8. Processo de execução para Codex/Claude

1. Ler este roadmap e `git status`, confirmar branch/HEAD reais e upstream, procurar implementações e PRs existentes antes de duplicar trabalho.
2. Selecionar item de maior prioridade desbloqueado. Abrir issue (ou ligar existente) com ID, escopo, evidência, dependências e critérios. PR pequena/atómica; paralelizar apenas trabalho independente sem conflito.
3. Reproduzir bug, adicionar teste, implementar, validar lint/test/build e segurança; não reduzir cobertura para passar CI. Corrigir erros observados e acrescentar regressão.
4. Rever diffs, comentários CI/CodeRabbit/revisores; resolver findings válidos, justificar falsos positivos, reexecutar verificações. Não gastar créditos nem subscrever serviços.
5. Só fazer merge com checks e critérios do item satisfeitos. Hardware em falta = BLOCKED/validação pendente, não DONE; documentação pode avançar em paralelo.
6. Atualizar este roadmap após trabalho real: estado, issue/PR, SHA merge, comandos executados/resultados, limitações, evidência de hardware quando aplicável, riscos e próximas tarefas. Não marcar fases fechadas por percentagens estimadas.
7. Não modificar upstream sem autorização, não apagar código/branches de terceiros e não publicar dados VIN/capturas identificáveis. Preservar licença e fontes de dados.

### Definition of Done

- [ ] Critérios específicos satisfeitos; testes de regressão adicionados e resultados anexados.
- [ ] CI exigido verde no SHA da PR e estado pós-merge verificado; sem panics/erros ocultados introduzidos.
- [ ] Revisão independente; findings críticos/altos resolvidos; documentação, i18n, compatibilidade e licenças atualizadas conforme impacto.
- [ ] Teste físico quando necessário; indicação clara de bloqueios e funcionalidades não verificadas.
- [ ] Roadmap e issue ligados ao commit/PR; nenhum claim de suporte sem evidência.

## 9. Estado inicial e limitações

A auditoria foi estática, sobre árvore do repositório, ficheiros selecionados, PRs, issues e runs; não foi uma revisão linha a linha integral nem uma análise de logs de jobs, compilação ou teste físico. O comparativo provém da documentação pública do fornecedor e não de ensaios lado a lado. As prioridades e arquitetura são propostas técnicas a validar. O fork `croquete1/obdium` foi criado pelo proprietário e confirmado com a main inicial no SHA auditado em 25/11/2026. Este documento inaugura o roadmap canónico em `docs/ROADMAP.md`; não implementa correções, não configura CI e não comprova testes físicos. Todos os IDs permanecem abertos até haver evidência.
