import { Eye, EyeOff, Loader2, ShieldCheck, ShieldX } from "lucide-react";
import { useEffect, useState } from "react";
import { cmd } from "../lib/bridge";
import { toast, useAccounts, useT } from "../lib/store";
import type { Account } from "../lib/types";
import { Button, Field, Modal, TextField } from "./ui";

type ValidState = null | "ok" | "fail";

export function AccountModal({
  open,
  account,
  onClose,
}: {
  open: boolean;
  /** null — режим добавления */
  account: Account | null;
  onClose: () => void;
}) {
  const t = useT();
  const { add, update } = useAccounts();

  const [username, setUsername] = useState("");
  const [cookie, setCookie] = useState("");
  const [note, setNote] = useState("");
  const [showCookie, setShowCookie] = useState(false);
  const [busy, setBusy] = useState(false);
  const [validating, setValidating] = useState(false);
  const [valid, setValid] = useState<ValidState>(null);

  useEffect(() => {
    if (!open) return;
    setUsername(account?.username ?? "");
    setCookie("");
    setNote(account?.note ?? "");
    setShowCookie(false);
    setValid(null);
    setBusy(false);
    setValidating(false);
  }, [open, account]);

  const validate = async () => {
    if (!cookie.trim()) return;
    setValidating(true);
    setValid(null);
    try {
      const res = await cmd<{ ok: boolean; username: string | null }>("validate_cookie", {
        cookie: cookie.trim(),
      });
      setValid(res.ok ? "ok" : "fail");
      if (res.ok && res.username && !username.trim()) setUsername(res.username);
    } catch {
      setValid("fail");
    } finally {
      setValidating(false);
    }
  };

  const submit = async () => {
    if (!cookie.trim() && !account) return;
    setBusy(true);
    try {
      if (account) {
        await update(account.id, {
          username: username.trim() || account.username,
          note,
          cookie: cookie.trim() || undefined,
        });
        toast("success", t("toast.accountUpdated"));
      } else {
        await add({
          username: username.trim(),
          cookie: cookie.trim(),
          note,
        });
        toast("success", t("toast.accountAdded"));
      }
      onClose();
    } catch (e) {
      toast("error", String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={account ? t("accounts.editTitle") : t("accounts.addTitle")}
      width={480}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t("action.cancel")}
          </Button>
          <Button
            variant="primary"
            disabled={busy || (!account && !cookie.trim())}
            icon={busy ? <Loader2 size={15} className="animate-spin" /> : undefined}
            onClick={() => void submit()}
          >
            {account ? t("action.save") : t("action.add")}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <Field id="acc-username" label={t("accounts.field.username")} hint={t("accounts.field.username.hint")}>
          <TextField
            id="acc-username"
            value={username}
            placeholder="e.g. Nexus_Builder"
            onChange={(e) => setUsername(e.target.value)}
          />
        </Field>

        <Field
          id="acc-cookie"
          label={t("accounts.field.cookie")}
          hint={t("accounts.field.cookie.hint")}
        >
          <div className="relative">
            <TextField
              id="acc-cookie"
              type={showCookie ? "text" : "password"}
              value={cookie}
              spellCheck={false}
              autoComplete="off"
              placeholder="_|WARNING:-DO-NOT-SHARE-THIS…"
              className="pr-10 font-mono text-[12px]"
              onChange={(e) => {
                setCookie(e.target.value);
                setValid(null);
              }}
            />
            <button
              type="button"
              aria-label={showCookie ? t("action.hide") : t("action.show")}
              onClick={() => setShowCookie((s) => !s)}
              className="absolute right-2.5 top-1/2 -translate-y-1/2 text-faint transition-colors hover:text-ink"
            >
              {showCookie ? <EyeOff size={16} /> : <Eye size={16} />}
            </button>
          </div>
        </Field>

        <div className="flex items-center gap-2">
          <Button
            size="sm"
            icon={
              validating ? (
                <Loader2 size={13} className="animate-spin" />
              ) : valid === "ok" ? (
                <ShieldCheck size={13} className="text-ok" />
              ) : valid === "fail" ? (
                <ShieldX size={13} className="text-err" />
              ) : undefined
            }
            disabled={!cookie.trim() || validating}
            onClick={() => void validate()}
          >
            {validating ? t("accounts.validating") : t("accounts.validate")}
          </Button>
          {valid === "ok" && <span className="text-xs text-ok">{t("accounts.valid.ok")}</span>}
          {valid === "fail" && <span className="text-xs text-err">{t("accounts.valid.fail")}</span>}
        </div>

        <Field id="acc-note" label={t("accounts.field.note")}>
          <TextField
            id="acc-note"
            value={note}
            onChange={(e) => setNote(e.target.value)}
          />
        </Field>
      </div>
    </Modal>
  );
}
