# Novidades do Lamina

As notas de cada versão, em linguagem simples. O Lamina mostra estas notas em "O que há de novo"
depois de atualizar, e a release copia daqui a seção da versão (`cargo xtask versao`). Versão nova
entra no topo, como `## [0.1.0-beta.5] - 2026-10-24`; o que ainda não saiu fica em
`## [Não lançado]`, que o Lamina não mostra.

Cada item diz o que mudou e como ver a mudança, em linguagem de fotógrafo:
`- **O que mudou.** Explicação curta. Experimente: o que fazer para ver.` O `cargo xtask versao`
avisa (sem impedir a versão) quando um item não tem "Experimente:". No fim do item, um comentário
que a página da versão não mostra põe a ilustração (um arquivo de `novidades/`) e o comando do
botão "Experimente agora": `<!-- imagem: atalhos.png; comando: ajuda.atalhos -->`.

## [Não lançado]

- **Uma tela inicial nova.** O último álbum aberto vem grande, e um clique em "Continuar" volta para a lâmina
  em que você parou. Os outros ficam embaixo, com os fixados primeiro. Experimente: feche o álbum pelo botão
  Início e veja o álbum esperando por você.

## [0.1.0-beta.8] - 2026-10-31

Esta versão traz layouts que enchem a lâmina até a borda, põe as fotos maiores primeiro nas sugestões, dá mais espaço à lâmina numa janela redesenhada e deixa o Lamina mais leve.

- **Layouts sem borda.** De 2 a 24 fotos, agora há layouts em que as fotos vão até a borda do papel,
  separadas só pelo espaço que você escolheu para o álbum; com espaço 0, ficam coladas. Eles cortam
  até 35% de cada foto, ou até 15% com o corte "Pouco". Com o corte "Nada", ficam desligados. Dá para
  desligar no álbum novo, em "Layouts sem borda". Experimente: abra a faixa de layouts com a
  tecla L e veja os que têm "Sem borda" no nome.
  <!-- imagem: layouts-sem-borda.png; comando: ver.faixa-layouts -->
- **As fotos maiores vêm primeiro.** As sugestões começam pelos layouts em que as fotos ficam maiores,
  e os que deixam muito branco vão para o fim. Por isso, ao arrastar ou tirar uma foto, a lâmina tende
  a se refazer mais cheia, com mais recorte. Na montagem automática, depois de duas lâminas até a borda
  vem uma mais calma. Experimente: arraste uma foto para uma lâmina e aperte ↓ para ver as outras opções.
  <!-- imagem: fotos-maiores-primeiro.png -->
- **A lâmina ganhou espaço.** Lâminas e Sequência viraram abas de uma faixa só, as fotos abrem com uma
  linha e a faixa encolhe em janela baixa. O painel da direita ficou organizado em Álbum, Impressão e
  Lâmina, os limites de páginas aparecem na barra de cima, e uma sombra na tela marca a dobra, sem ir
  para a impressão. Cinzas neutros em volta ajudam a julgar a cor das fotos. Experimente: ligue e
  desligue a sombra em Ver, Sombra da dobra.
  <!-- imagem: lamina-com-mais-espaco.png; comando: ver.sombra-dobra -->
- **O Lamina ficou mais leve.** Exportar com fotos de 24 MP caiu de cerca de 3 GB para 0,8 GB de memória,
  em troca de levar de 13% a 29% mais tempo. Importar gasta 57% menos memória no mesmo tempo, e editar
  não copia mais o álbum. Os arquivos exportados continuam idênticos. Experimente: exporte um
  álbum grande com outros programas abertos.
  <!-- imagem: mais-leve.png; comando: arquivo.exportar -->
- **Fotos que não apareciam.** Em computadores em que o programa não consegue usar o OpenGL, as lâminas
  vizinhas e as fotos nítidas não chegavam à tela, e parte da lâmina podia ficar cinza. Em qualquer
  computador, uma foto também podia se perder ao chegar entre dois desenhos da janela. Fechar esta
  janela de novidades agora também devolve a memória das ilustrações. Experimente: passe de lâmina com
  → e veja as fotos ficarem nítidas.

## [0.1.0-beta.7] - 2026-10-29

Esta versão traz as fontes do computador nos textos, reencontra as fotos que mudaram de lugar, tira fotos do álbum pela grade, simplifica a escolha do tamanho, deixa editar os textos que já estão na lâmina e leva fotos de uma lâmina para outra pela Sequência.

- **As fontes do computador nos textos.** O campo Fonte da aba Quadro abre uma lista com busca: em
  cima, as 6 fontes do Lamina; embaixo, todas as fontes instaladas no seu computador, cada nome
  escrito na própria fonte. O texto sai na fonte escolhida na tela e na exportação. Se o álbum for
  aberto num computador que não tem a fonte, o texto aparece em Inter, com aviso. Experimente:
  selecione um texto, clique em Fonte e digite parte do nome de uma fonte.
  <!-- imagem: fontes-do-computador.png -->
- **As fotos que mudaram de lugar são reencontradas.** Se você levou a pasta de fotos para um SSD
  externo ou outra pasta, o Lamina avisa ao abrir o álbum qual pasta sumiu. Clique em "Mostrar
  onde está…", aponte a pasta nova uma vez, e as fotos voltam sozinhas, mesmo com milhares. Com o
  disco desconectado, ele avisa; conecte o disco e clique em "Tentar de novo". Se você renomear ou
  mover a pasta com o álbum aberto, o Lamina percebe em segundos, diz o que aconteceu e traz as
  fotos com um clique em "Usar o lugar novo". Experimente: com o álbum aberto, renomeie a pasta
  das fotos no Explorador e volte para o Lamina.
  <!-- imagem: reencontrar-fotos.png; comando: arquivo.procurar-fotos -->
- **Fotos de uma lâmina para outra pela Sequência.** Na faixa Sequência, arraste uma foto para
  outra lâmina ou para outra posição: uma linha azul mostra onde ela vai entrar, e as lâminas que
  mudam são refeitas sozinhas. Esc desiste. Experimente: arraste uma foto da lâmina 1 para depois
  do corte da lâmina 2.
  <!-- imagem: fotos-na-sequencia.png -->
- **As setas ↑ e ↓ trocam o layout, e ← e → passam de lâmina.** As setas para cima e para baixo
  trocam o layout da lâmina inteira; as setas para os lados vão para a próxima lâmina e para a
  anterior, como o Page Down e o Page Up, também com a faixa de layouts aberta. A troca do layout
  de uma página só saiu do teclado e do menu Layout. No modo organizar, as setas continuam movendo
  os quadros. Experimente: aperte ↓ para trocar o layout e → para ir à próxima lâmina.
- **Tirar uma foto refaz o layout.** Com Delete, a foto sai junto com o quadro, e a página ganha a
  melhor arrumação para as fotos que ficaram, sem buraco vazio. Experimente: selecione uma foto
  numa lâmina com três e aperte Delete.
  <!-- imagem: tirar-foto-refaz.png -->
- **Remover fotografias do álbum e achar o arquivo.** Clique com o botão direito numa foto da
  faixa Fotografias ou da lâmina: "Remover fotografia do álbum" tira a foto do álbum, e na faixa
  "Mostrar no Explorador" (no Mac, "Mostrar no Finder") abre a pasta com o arquivo já selecionado.
  Com a grade em foco, Delete faz o mesmo. Se a foto estiver numa lâmina, o Lamina pergunta antes e refaz a lâmina. O arquivo nunca é apagado do computador,
  e Ctrl+Z desfaz tudo. Experimente: clique com o botão direito numa foto da faixa Fotografias.
- **Escolher o tamanho do álbum ficou mais simples.** No álbum novo, o tamanho do último álbum já
  vem marcado. Os três formatos aparecem como o álbum fechado e, embaixo, a lâmina aberta do
  tamanho escolhido, com um resumo de uma linha (tamanho, páginas, DPI e sangria). Os nomes são a
  medida, como 30x30cm e 20x15cm. Dá para escolher a encadernadora numa lista com busca, e o
  Personalizado ficou na horizontal, com os números da gráfica em "Ajustes da gráfica". Experimente:
  crie um álbum novo.
  <!-- comando: arquivo.novo -->
- **Editar os textos que já estão na lâmina.** Um clique num texto mostra o campo dele na aba
  Quadro, mesmo com o painel escondido, e o texto fica sempre por cima das fotos. "Editar texto"
  também está no menu Lâmina e no botão direito. Experimente: clique num texto do álbum de exemplo
  e escreva.
  <!-- comando: lamina.editar-texto -->
- **Limpar as lâminas de uma vez.** No menu Lâmina, "Tirar as fotos de todas as lâminas…" deixa os
  quadros vazios para diagramar de novo, e "Apagar todas as lâminas…" deixa o álbum sem lâminas;
  as fotos continuam importadas, e Ctrl+Z desfaz. Experimente: abra o menu Lâmina.
- **Sair pelo menu Arquivo.** Arquivo → Sair (Ctrl+Q; no Mac, Cmd+Q) fecha o Lamina, perguntando
  antes se quer salvar. Experimente: abra o menu Arquivo.
- **Setas nos campos de número.** Em qualquer campo de número (medidas, DPI, páginas, posição do
  quadro, fonte, ângulo, escala), ↑ e ↓ sobem e descem o valor, e com Shift andam de dez em dez.
  Experimente: selecione uma foto, clique no ângulo e aperte ↑.
- **O painel da direita na altura toda.** O painel desce até o fim da janela, e as faixas de baixo
  ficam ao lado dele. Experimente: abra um álbum e veja o painel da direita.
- **Botões de girar menores.** Girar 90°, ajustar 0,1° e voltar a 0° viraram ícones pequenos, numa
  linha só ao lado do ângulo, cada um com o balão do que faz. Experimente: selecione uma foto e
  passe o mouse nos ícones de girar.
- **Tamanhos da ProAlbuns.** Os 14 álbuns da tabela da ProAlbuns entraram, dos quadrados de
  15x15cm a 30x30cm aos horizontais de até 45x30cm, com a sangria que ela pede. Eles aparecem com a
  encadernadora ProAlbuns escolhida; em "Tamanhos padrão" ficam os do Lamina. Experimente: crie um
  álbum novo e escolha a encadernadora ProAlbuns.
- **Controle por IA mais certeiro.** A IA monta a primeira lâmina de um álbum em branco, dá nome ao
  álbum novo e troca o nome dele, mostra quanto cada sugestão de layout preenche, e explica por que
  uma importação ficou vazia. Experimente: peça à IA para criar um álbum com um nome.

## [0.1.0-beta.6] - 2026-10-28

Esta versão corrige o Lamina que fechava sozinho, guarda uma cópia de segurança do trabalho e traz temperatura e matiz, controles deslizantes, lâminas que mudam de lugar arrastando e uma barra única no modo organizar.

- **Temperatura e matiz da foto.** Dois controles novos na seção Foto: a temperatura deixa a foto
  mais quente (amarelada) ou mais fria (azulada), e o matiz puxa para o magenta ou para o verde. A
  cor vale em todos os quadros onde a foto aparece e sai igual na exportação; clique duplo no
  controle volta ao normal. Experimente: selecione uma foto e arraste a Temperatura para a direita.
  <!-- imagem: temperatura-e-matiz.png -->
- **Girar e escala com controles deslizantes.** A foto gira de −180° a 180°, com os botões −90° e
  +90°, e a escala tem um controle de 100% a 800%, fino perto de 100%. No modo organizar, o zoom da
  lâmina também ganhou um controle. Experimente: selecione uma foto e clique em +90°.
  <!-- imagem: girar-e-escala.png -->
- **As lâminas mudam de lugar arrastando na faixa.** Na faixa Lâminas, aperte uma lâmina e arraste:
  ela sai do lugar e segue o mouse, as outras abrem espaço, e ao soltar ela entra no vão. A faixa
  rola sozinha perto das bordas, e Esc desiste. Experimente: arraste a lâmina 2 para depois da 4.
  <!-- imagem: reordenar-laminas.png -->
- **As faixas de baixo com um botão só.** O botão com o desenho da janela, no canto de cima,
  esconde e mostra as faixas Lâminas, Sequência e Fotografias de uma vez (Alt+0), e lembra quais
  estavam abertas; um clique no nome de cada faixa recolhe só ela. A faixa Lâminas agora aparece
  sempre, e a "linha do tempo" passou a se chamar Sequência. O painel da direita fica mais largo ou
  mais estreito arrastando a borda esquerda dele. Experimente: clique no botão das faixas de baixo.
  <!-- imagem: faixas-de-baixo.png; comando: ver.faixas -->
- **Uma barra só no modo organizar.** As ferramentas, o zoom, Alinhar, Igualar, Distribuir e a
  posição e o tamanho do quadro ficam numa barra em cima da lâmina, e nada flutua mais por cima
  dela. Alinhar e Igualar acendem com dois quadros selecionados, e Distribuir com três.
  Experimente: entre no modo organizar e selecione dois quadros com Ctrl+clique.
  <!-- imagem: barra-organizar.png; comando: lamina.organizar -->
- **Trocar o nome do álbum.** Clique duas vezes no nome do álbum, no canto de cima à esquerda,
  digite o nome novo e aperte Enter; Esc desiste e Ctrl+Z volta o nome de antes. O arquivo continua
  com o mesmo nome. Experimente: clique duas vezes no nome do álbum de exemplo.
  <!-- imagem: renomear-album.png; comando: arquivo.renomear -->
- **A janela maximiza de primeira no Windows.** O Lamina abre ocupando a tela toda, e maximizar
  pelo botão, pelo clique duplo no título ou por Win+↑ funciona na primeira vez, sem a lâmina
  demorar para crescer. Experimente: restaure a janela e maximize de novo.
- **As miniaturas dos álbuns recentes não somem.** Uma foto apagada do disco não deixa mais o cartão
  da tela inicial sem miniatura: o quadro dela aparece cinza e o resto da lâmina sai normal.
  Experimente: volte ao Início com Ctrl+W e veja os cartões.
- **O Lamina não fecha mais sozinho ao diagramar.** Criar lâminas arrastando fotos podia fechar o
  Lamina depois de alguns minutos, principalmente no Mac; a conta que causava isso foi corrigida, e
  as janelas de abrir, salvar e importar agora ficam presas à janela do Lamina. Experimente:
  arraste várias fotos para o "+ Lâmina nova", uma de cada vez, e siga diagramando.
- **Uma cópia de segurança do trabalho não salvo.** Enquanto você diagrama, o Lamina guarda uma
  cópia do álbum a cada minuto. Se ele fechar sem salvar, na próxima abertura ele pergunta se você
  quer recuperar, e o álbum volta na lâmina em que você estava. O seu arquivo só muda quando você
  salva. Experimente: faça uma alteração e salve com Ctrl+S; a cópia some sozinha.
- **A barra de cima de cada lâmina fica parada sob o mouse.** Os botões Organizar, Layouts,
  Espelhar e Texto e a alça agora dão para clicar, e a barra espera meio segundo antes de sumir
  quando o mouse sai. Experimente: passe o mouse sobre a lâmina, suba até a barra e clique no
  lápis.

## [0.1.0-beta.5] - 2026-10-22

Esta versão põe as lâminas lado a lado, traz a faixa de layouts, o painel de fotos em grade e o álbum que começa em branco, e corrige o Lamina que fechava no Mac ao importar muitas fotos.

- **Os álbuns recentes inteiros, com a primeira lâmina.** Na tela inicial, cada álbum aparece num
  cartão completo, com a primeira lâmina, o nome, quando foi aberto e quantas lâminas e fotos tem;
  um álbum ainda sem lâminas aparece como um álbum em branco com o nome. Experimente: abra um
  álbum, feche com Ctrl+W e veja o cartão dele na tela inicial.
- **As lâminas lado a lado.** As lâminas vizinhas aparecem dos lados da que você está editando, e a
  roda do mouse ou dois dedos no touchpad passeiam pelo álbum de lado. Um clique numa vizinha a
  traz para o meio; passando o mouse, aparecem os atalhos dela e a alça para mudar a lâmina de
  lugar. Experimente: role a roda do mouse sobre a lâmina e clique na vizinha da direita.
  <!-- imagem: rolo.png -->
- **Fotos de uma lâmina para outra.** Arraste a foto de um quadro para outra lâmina: num quadro
  vazio ela entra, num quadro com foto as duas trocam de lugar, e fora dos quadros ela entra como
  uma foto a mais e a lâmina se rearruma. O quadro de onde ela saiu fica vazio, e um Ctrl+Z desfaz
  nas duas lâminas. Experimente: arraste uma foto da lâmina para um quadro da vizinha ao lado.
  <!-- imagem: fotos-entre-laminas.png -->
- **A linha do tempo acompanha você.** As fotos da lâmina em que você está ficam destacadas e no
  meio da linha do tempo, e um clique numa foto leva à lâmina dela. Experimente: passe para a
  próxima lâmina com Page Down e clique numa foto da linha do tempo.
  <!-- imagem: linha-do-tempo.png -->
- **Faixa de layouts.** Aperte L (ou o botão Layouts sobre a lâmina) e os layouts descem numa
  faixa por cima da lâmina; pare o mouse num deles e a lâmina já mostra as fotos nos lugares novos,
  sem gravar nada. Clique para ficar com ele; Esc ou um clique fora fecha sem mudar nada.
  Experimente: aperte L e passe o mouse pelos layouts.
  <!-- imagem: faixa-de-layouts.png; comando: ver.faixa-layouts -->
- **Painel de fotos em grade.** O painel mostra várias fileiras e rola para baixo; o zoom do
  cabeçalho (ou Ctrl+roda) deixa as miniaturas menores ou maiores, a divisória dá mais altura ao
  painel e um retângulo arrastado no vão entre as fotos seleciona várias de uma vez. Experimente:
  arraste um retângulo entre as fotos e veja quantas foram selecionadas.
  <!-- imagem: painel-de-fotos.png -->
- **O álbum começa em branco.** O Novo álbum abre com todas as fotos no painel e nenhuma lâmina:
  você monta aos poucos, soltando as fotos na lâmina vazia. Quer tudo de uma vez? "Montar o
  álbum…", no cabeçalho do painel de fotos, monta as fotos que sobraram, e um Ctrl+Z desfaz tudo.
  Experimente: crie um álbum com uma pasta de fotos e solte três fotos na lâmina vazia.
  <!-- imagem: album-em-branco.png; comando: lamina.montar-album -->
- **Muitas fotos sem travar.** Importar 1.000 fotos de uma vez não fecha mais o Lamina: ele lê
  poucas fotos por vez, usa bem menos memória e as miniaturas vão aparecendo sem travar a janela.
  Experimente: Arquivo → "Importar fotos…" e escolha uma pasta grande.
  <!-- imagem: muitas-fotos.png; comando: arquivo.importar -->
- **Janela enxuta.** A barra de cima ficou só com o essencial, e o painel da direita começa
  escondido, deixando a lâmina maior; ele aparece sozinho quando você ajusta uma foto ou escreve um
  texto. O quadro vazio ganhou um X na tela, que não sai na exportação. Experimente: clique em
  Painel para mostrar ou esconder; o Lamina lembra a escolha.
  <!-- imagem: janela-enxuta.png; comando: ver.painel -->
- **Atalhos do teclado.** Ajuda → "Atalhos do teclado…" mostra todos os atalhos e os gestos do
  mouse, com busca. Experimente: aperte F1 e procure “trocar”.
  <!-- imagem: atalhos.png; comando: ajuda.atalhos -->
- **Lupa e tamanho real no modo organizar.** A lupa (Z) aproxima no ponto clicado, até 8×; o 1:1
  mostra a lâmina no tamanho de verdade, calibrado com um cartão de banco, e Ajustar (0) volta à
  lâmina inteira. Cada botão da paleta diz o que faz e a tecla. Experimente: aperte O para
  organizar a lâmina, depois Z, e clique num rosto.
  <!-- imagem: zoom.png; comando: lamina.organizar -->
- **Opacidade, borda e cantos da foto.** Com uma foto clicada, a seção Foto ganhou opacidade,
  borda, cor da borda e cantos, que mudam a lâmina na hora; a foto translúcida deixa ver o fundo, e
  o JPEG e o PDF saem iguais à tela. Experimente: clique numa foto sobre um fundo colorido e leve a
  opacidade para 50%.
  <!-- imagem: opacidade.png -->
- **Clique duplo volta ao neutro.** Nos controles de endireitar, brilho e contraste da foto, dois
  cliques no seletor levam o valor de volta ao zero, e um Ctrl+Z desfaz só isso. Experimente: em
  "Ajustar a foto…", arraste o brilho e dê dois cliques no seletor.
  <!-- imagem: seletor-neutro.png; comando: foto.ajustar -->
- **No Mac, o Lamina na pasta Aplicativos.** Instalado, o Lamina fica na pasta Aplicativos, como os
  outros programas, e dois cliques num arquivo .lamina abrem o álbum, com o Lamina aberto ou
  fechado. Experimente: no Finder, dê dois cliques num álbum .lamina.
  <!-- imagem: mac.png -->
- **O que há de novo.** Depois de atualizar, o Lamina mostra o que mudou desde a sua versão, com o
  que experimentar em cada novidade, numa animação curta. Experimente: Ajuda → "O que há de novo…".
  <!-- imagem: novidades.png; comando: ajuda.novidades -->

## [0.1.0-beta.4] - 2026-10-20

Esta é a versão da primeira rodada de testes com fotógrafos.

- **Tela inicial com os seus álbuns.** Os últimos álbuns aparecem em cartões com miniatura, e um
  clique abre o álbum na lâmina em que você parou. Dá para fixar os favoritos no topo; Ctrl+W ou o
  botão Início voltam para essa tela.
- **Novo álbum em três passos.** Escolha o tamanho (formatos desenhados, com os tamanhos de
  mercado, ou medidas próprias em mm, cm ou polegadas), depois como as fotos entram (cortar nada,
  pouco ou à vontade, com a cor do fundo e da borda num seletor) e por fim as fotos e onde salvar.
  O álbum já é gravado na pasta e dá para começar sem fotos.
- **Movimento.** Trocar o layout, espelhar, trocar fotos e desfazer mostram cada foto deslizando
  para o lugar novo, e ir para outra lâmina desliza na direção certa. Prefere sem animação?
  Visualizar → "Reduzir movimento".
- **Mais rápido.** Cada mudança aparece na tela em menos de 40 milésimos de segundo, e as fotos
  ficam nítidas sem travar a navegação.
- **Arrastar com retorno.** Arrastar uma foto mostra a parte dela que está no quadro, acende o
  quadro de destino e a foto "pousa" ao soltar; reordenar lâminas abre espaço entre as vizinhas.
- **Ajustes da foto.** Endireite uma foto torta com precisão de 0,1° (até 45°), com a grade de
  referência, e ajuste brilho, contraste e preto e branco. A foto nunca mostra canto vazio ao
  girar e sai igual na exportação. Clique direito na foto → "Ajustar a foto…"; "Restaurar a foto"
  volta ao original.
- **Painel Detalhes.** A antiga aba Info mostra o álbum quando nada está selecionado e, com uma
  foto, os dados da câmera (velocidade, abertura, ISO e lente). Alt+4 esconde o painel e dá mais
  espaço para a lâmina.
- **Organizar a lâmina à mão.** Dois cliques fora dos quadros (ou a tecla O) abrem a lâmina em
  destaque: selecione vários quadros, mova, amplie até sangrar, alinhe, iguale e distribua, com
  réguas, zoom e o aviso "= 9 cm" quando as medidas batem. Esc conclui.
- **Importar fotos num álbum aberto.** Arquivo → "Importar fotos…" (Ctrl+I), o botão "+ Fotos" ou
  arrastar fotos e pastas do Explorador para a janela. Fotos repetidas não entram duas vezes, e
  Ctrl+Z desfaz.
- **Soltar fotos numa lâmina.** Arraste fotos da faixa para o "+ Lâmina nova", entre duas lâminas
  ou para uma lâmina pronta, e o Lamina monta o layout com elas.
- **Programa menor.** O instalador ficou mais leve, e as exportações saem iguais.
- **Atenção:** álbuns salvos nesta versão não abrem nas betas anteriores. Os álbuns antigos abrem
  normalmente aqui.
- **Experimente sem medo.** Na tela de boas-vindas, "Experimentar com o álbum de exemplo" abre um
  álbum pronto, com fotos livres de uma viagem. Mexa à vontade: ele é uma cópia só sua.
- **Mande sugestões e problemas sem criar conta.** Ajuda → "Enviar sugestão ou problema…" abre um
  formulário curto no navegador, já com a versão do Lamina. Se algo der errado, Ajuda → "Abrir
  pasta de registros" mostra o arquivo para anexar. O Lamina não envia nada sozinho.
- **Erros que dá para entender.** Quando algo dá errado, a mensagem diz o que aconteceu e o que
  fazer agora. O texto técnico fica em "Mostrar detalhes".
- **Nada de tela travada.** Se uma tarefa em segundo plano falhar, o Lamina volta a responder e
  avisa, em vez de ficar parado em "Exportando…".

## [0.1.0-beta.3] - 2026-10-19

- **O Lamina no Claude Desktop.** Baixe o `Lamina-Windows.mcpb` (ou o `Lamina-macOS.mcpb`) desta
  página e dê dois cliques: o Claude passa a abrir os seus álbuns, apontar os problemas de
  impressão, trocar layouts, fotos, recortes, textos e a aparência, e exportar para a gráfica.
  Você pede em português comum. Cada mudança dele aparece como "IA: …" e pode ser desfeita, e o
  álbum só é salvo quando você pede.
- **Suas fotos e a LGPD.** Com "Permitir que a IA veja as fotos" ligada, as prévias e as
  miniaturas das fotos dos seus clientes vão para o provedor da IA (a Anthropic, no Claude).
  Desligue essa opção, nas configurações da extensão, se o contrato com o cliente pedir: o Claude
  continua trabalhando só com os dados das fotos. Mais detalhes na seção "Usar com o Claude
  Desktop" do README do Lamina.
- O pacote ainda não tem assinatura digital: o Claude Desktop pode mostrar um aviso ao instalar.
- No Mac, o pacote ainda não foi testado num computador de verdade. Se der problema, conte para
  a gente.

## [0.1.0-beta.2] - 2026-10-19

- A tela Sobre mostra só o Lamina.
- Versão para conferir a atualização automática a partir da 0.1.0-beta.1.

## [0.1.0-beta.1] - 2026-10-19

Primeira versão de teste do Lamina, para os testadores.

- Álbum automático a partir de uma pasta de fotos, com troca de layouts, recorte, texto, modelos
  e presets de impressão.
- Exportação em JPEG, TIFF e PDF/X-4 para a gráfica, com a conferência antes de exportar.
- Instalador para Windows, Mac e Linux, e instalação pelo terminal sem o aviso de segurança.
- Atualização automática: o Lamina avisa quando há versão nova e se atualiza sozinho.
- Duplo clique num álbum `.lamina` abre o Lamina (Windows e Linux).
