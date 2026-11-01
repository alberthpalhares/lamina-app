# Instalar o Lamina

O Lamina é um programa gratuito para diagramar álbuns de fotos. Este guia mostra como instalar no
Windows, no Mac e no Linux, em poucos minutos.

**Antes de começar:** na página da versão, o começo das notas tem o link do arquivo do seu sistema.
Se preferir, role até **Assets** (arquivos); se a lista estiver curta, clique em **Show all
assets** para ver todos. Os nomes têm `beta` quando é uma versão de teste, e o selo **Pre-release**
da página é normal numa versão de teste. Os outros arquivos da lista (`.nupkg`, `.json`, `RELEASES`)
são das atualizações automáticas: não precisa baixar.

| Sistema | Arquivo para baixar |
|---|---|
| Windows 10 ou 11 | o que termina em `Setup.exe` (por exemplo, `Lamina-win-beta-Setup.exe`) |
| Mac (Intel ou Apple) | o que termina em `Setup.pkg` (por exemplo, `Lamina-osx-beta-Setup.pkg`) |
| Linux | o que termina em `.AppImage` (por exemplo, `Lamina-linux-beta.AppImage`) |

## Por que o sistema avisa na primeira vez

![Os avisos do Windows e do Mac e os cliques que liberam o Lamina](../../../site/midia/instalar.png)

O Lamina não paga a assinatura de desenvolvedor da Microsoft e da Apple. Por isso, na primeira
vez, o Windows e o Mac dizem que não reconhecem o programa. **O aviso não quer dizer vírus:** o
código do Lamina é aberto, cada instalador é montado pelo GitHub Actions a partir dele, e cada
versão publica o arquivo `SHA256SUMS` para você conferir o arquivo baixado (veja o fim deste
guia). Depois da instalação, as atualizações chegam pelo próprio Lamina, sem esse aviso.

Há três caminhos em cada sistema:

- **Plano A, sem aviso:** o comando do fim deste guia, colado no PowerShell ou no Terminal.
- **Plano B, pelos cliques:** baixar o arquivo e passar pelo aviso, como mostram as seções abaixo.
- **Plano C, de reserva:** se o botão de liberar não aparecer (Windows) ou o Mac disser que o
  Lamina está danificado.

## Windows

1. Se o navegador avisar que o arquivo **não é baixado com frequência** ou que **pode ser
   perigoso**, ele não apagou nada ainda:
   - **Edge:** passe o mouse no aviso, clique nos **três pontinhos** → **Manter** → **Mostrar mais**
     → **Manter mesmo assim**;
   - **Chrome:** clique em **Manter** (ou **Baixar arquivo suspeito**).
2. Abra o arquivo `Setup.exe` que você baixou (clique duas vezes nele, na pasta Downloads).
3. Se aparecer uma janela azul **"O Windows protegeu o computador"**:
   1. clique em **Mais informações** (o texto pequeno, embaixo da mensagem);
   2. clique no botão **Executar assim mesmo**.
4. A instalação leva poucos segundos e não pede senha de administrador. No fim, o Lamina abre
   sozinho.
5. Nas próximas vezes, abra pelo atalho **Lamina** na área de trabalho ou no menu Iniciar.

**Plano C (Windows):** se o botão **Executar assim mesmo** não aparecer (alguns computadores de
empresa escondem), clique com o botão direito no `Setup.exe` → **Propriedades** → na parte de
baixo, em Segurança, marque **Desbloquear** → **OK**, e abra de novo. Pelo PowerShell, o mesmo:
`Unblock-File "$HOME\Downloads\Lamina-win*-Setup.exe"`.

## Mac

1. Abra o arquivo `Setup.pkg` que você baixou (na pasta Transferências) e siga o instalador. O Mac
   pede a sua senha para instalar: é a senha de entrar no Mac. O Lamina fica na pasta
   **Aplicativos**, e as próximas atualizações não pedem a senha.
2. Se o Mac disser que **não pode verificar o desenvolvedor** ou que o Lamina **não foi aberto**:
   1. clique em **OK** (ou **Concluído**);
   2. abra **Ajustes do Sistema** (o ícone de engrenagem) → **Privacidade e Segurança**;
   3. role até o fim e, ao lado da mensagem sobre o Lamina, clique em **Abrir mesmo assim**;
   4. confirme com a sua senha do Mac, se ele pedir.
3. Isso pode acontecer duas vezes: ao abrir o instalador e ao abrir o Lamina pela primeira vez.
   Depois, abra o Lamina pela pasta **Aplicativos** ou pelo Launchpad, sem aviso. No macOS 15
   (Sequoia) ou mais novo, o antigo atalho "botão direito → Abrir" não libera mais: use os Ajustes.
4. Para abrir um álbum, dê dois cliques no arquivo `.lamina` no Finder. Se o Finder ainda mostrar
   outro programa, clique com o botão direito no arquivo → **Abrir com** → **Lamina**.

**Plano C (Mac):** se o Mac disser que o Lamina **está danificado** ou continuar sem abrir, abra
o Terminal (Launchpad → Outros → Terminal), cole o comando abaixo e abra o Lamina de novo. Ele tira
do Lamina a marca de "baixado da internet":

```sh
xattr -dr com.apple.quarantine /Applications/Lamina.app
```

## Linux

1. Baixe o arquivo `.AppImage` e guarde numa pasta sua (por exemplo, `Aplicativos` na sua pasta
   pessoal).
2. Clique com o botão direito no arquivo → **Propriedades** → **Permissões** e marque **Permitir
   executar o arquivo como programa** (em versões novas do Ubuntu, a opção se chama **Executável como
   programa**).
3. Clique duas vezes no arquivo para abrir. Na primeira vez, o Lamina cria o atalho no menu de
   aplicativos.
4. Se **nada acontecer** ao clicar duas vezes, falta um pacote do sistema. Abra o Terminal e rode
   `sudo apt install libfuse2` (no Ubuntu 24.04 ou mais novo: `sudo apt install libfuse2t64`).
   Ou instale pelo comando do fim deste guia.

## Primeiro uso

Na tela de boas-vindas, clique em **Álbum de exemplo**. Abre um álbum pronto,
com fotos livres, para você ver como o Lamina funciona: troque layouts com as setas ↑ e ↓, passe
de lâmina com ← e →, ponha um título, exporte. Nada que você fizer ali afeta as suas fotos.

Quando quiser, volte e escolha **Novo álbum…**: o Lamina monta o álbum a partir
de uma pasta de fotos.

## Atualizações

O Lamina procura versões novas sozinho, baixa em segundo plano e avisa com **Reiniciar e
atualizar**. Quem instalou uma versão de teste (beta) já recebe as próximas versões de teste;
para voltar só às versões estáveis, desligue **Ajuda → Receber versões de teste (beta)**.

## Encontrou um problema ou tem uma ideia?

No Lamina, use **Ajuda → Enviar sugestão ou problema…**. Abre um formulário curto no navegador,
sem precisar de conta. Se algo deu errado, anexe o registro: **Ajuda → Abrir pasta de registros**
mostra o arquivo `lamina.log`. Não anexe fotos de clientes.

## Para quem prefere o terminal (plano A)

Estes comandos instalam o Lamina sem o aviso do sistema (o antivírus continua examinando os
arquivos) e conferem o SHA-256 antes de instalar:

- **Windows (PowerShell):**
  `irm https://raw.githubusercontent.com/alberthpalhares/lamina-app/main/crates/lamina-app/dist/instalar.ps1 | iex`
  (para a versão de teste, rode antes `$env:LAMINA_BETA = 1`);
- **Mac e Linux:**
  `curl -fsSL https://raw.githubusercontent.com/alberthpalhares/lamina-app/main/crates/lamina-app/dist/instalar.sh | sh -s -- --beta`
  (a versão de teste; sem o `-s -- --beta`, a estável, quando houver). No Mac, o Lamina vai para
  a pasta **Aplicativos** sem pedir senha; se o seu usuário não puder gravar lá, vai para a pasta
  Aplicativos da sua pasta pessoal, e o comando avisa.

## Conferir o arquivo baixado

Na página da versão, o arquivo `SHA256SUMS` traz a impressão digital de cada instalador. Compare
com a do arquivo que você baixou:

- **Windows (PowerShell):** `Get-FileHash "$HOME\Downloads\Lamina-win*-Setup.exe"`
- **Mac:** `shasum -a 256 ~/Downloads/Lamina-osx*-Setup.pkg`
- **Linux:** `sha256sum ~/Downloads/Lamina-linux*.AppImage`

Se o antivírus acusar o Lamina, é quase sempre um alarme falso de programa sem assinatura: confira
o SHA-256, envie o arquivo para análise da Microsoft em
<https://www.microsoft.com/wdsi/filesubmission> e avise pelo formulário do Lamina.
