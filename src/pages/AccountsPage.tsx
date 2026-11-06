import { PlayCircle, Plus, Search, Users } from "lucide-react";
import { useMemo, useState } from "react";
import { AccountCard } from "../components/AccountCard";
import { AccountModal } from "../components/AccountModal";
import { Button, EmptyState, Modal, TextField } from "../components/ui";
import { toast, useAccounts, useT } from "../lib/store";
import type { Account } from "../lib/types";

export function AccountsPage() {
  const t = useT();
  const { accounts, loaded, remove, launchAll, terminate, instances } = useAccounts();

  const [query, setQuery] = useState("");
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<Account | null>(null);
  const [toDelete, setToDelete] = useState<Account | null>(null);
  const [toTerminate, setToTerminate] = useState<Account | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return accounts;
    return accounts.filter(
      (a) =>
        a.username.toLowerCase().includes(q) || a.note.toLowerCase().includes(q),
    );
  }, [accounts, query]);

  const runningCount = Object.keys(instances).length;

  return (
    <div className="flex h-full flex-col">
      {/* Заголовок + действия */}
      <div className="flex items-center gap-3 px-6 pb-1 pt-2">
        <div className="mr-auto min-w-0">
          <h1 className="truncate text-[22px] font-bold tracking-tight text-ink">
            {t("accounts.title")}
          </h1>
          <p className="truncate text-[13px] text-faint">
            {t("accounts.subtitle")} · {t("accounts.count", { n: accounts.length })}
          </p>
        </div>

        <Button
          icon={<PlayCircle size={16} />}
          disabled={accounts.length === 0}
          onClick={() => void launchAll()}
        >
          {t("action.launchAll")}
        </Button>
        <Button
          variant="primary"
          icon={<Plus size={16} />}
          onClick={() => {
            setEditing(null);
            setModalOpen(true);
          }}
        >
          {t("action.add")}
        </Button>
      </div>

      {/* Поиск */}
      <div className="px-6 pb-2">
        <div className="relative w-full max-w-sm">
          <Search
            size={15}
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-faint"
          />
          <TextField
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("accounts.search")}
            className="h-9 pl-9"
          />
        </div>
      </div>

      {/* Сетка аккаунтов */}
      <div className="ram-scroll flex-1 overflow-y-auto px-6 pb-6 pt-2">
        {loaded && accounts.length === 0 ? (
          <EmptyState
            icon={<Users size={24} />}
            title={t("accounts.empty.title")}
            hint={t("accounts.empty.hint")}
            action={
              <Button
                variant="primary"
                icon={<Plus size={16} />}
                onClick={() => {
                  setEditing(null);
                  setModalOpen(true);
                }}
              >
                {t("action.add")}
              </Button>
            }
          />
        ) : (
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
            {filtered.map((a) => (
              <AccountCard
                key={a.id}
                account={a}
                onEdit={() => {
                  setEditing(a);
                  setModalOpen(true);
                }}
                onDelete={() => setToDelete(a)}
                onTerminateAsk={() => setToTerminate(a)}
              />
            ))}
          </div>
        )}
      </div>

      {/* Модалка добавления/редактирования */}
      <AccountModal open={modalOpen} account={editing} onClose={() => setModalOpen(false)} />

      {/* Подтверждение удаления */}
      <Modal
        open={toDelete !== null}
        onClose={() => setToDelete(null)}
        title={t("accounts.deleteTitle")}
        width={420}
        footer={
          <>
            <Button variant="ghost" onClick={() => setToDelete(null)}>
              {t("action.cancel")}
            </Button>
            <Button
              variant="danger"
              onClick={() => {
                if (toDelete)
                  void remove(toDelete.id).then(() => toast("success", t("toast.accountDeleted")));
                setToDelete(null);
              }}
            >
              {t("action.delete")}
            </Button>
          </>
        }
      >
        <p className="text-sm leading-relaxed text-mut">
          {toDelete && t("accounts.deleteText", { name: toDelete.username })}
        </p>
      </Modal>

      {/* Подтверждение завершения клиента */}
      <Modal
        open={toTerminate !== null}
        onClose={() => setToTerminate(null)}
        title={t("accounts.terminateTitle")}
        width={420}
        footer={
          <>
            <Button variant="ghost" onClick={() => setToTerminate(null)}>
              {t("action.cancel")}
            </Button>
            <Button
              variant="danger"
              onClick={() => {
                if (toTerminate)
                  void terminate(toTerminate.id).then(() =>
                    toast("success", t("toast.clientTerminated")),
                  );
                setToTerminate(null);
              }}
            >
              {t("action.terminate")}
            </Button>
          </>
        }
      >
        <p className="text-sm leading-relaxed text-mut">
          {toTerminate && t("accounts.terminateText", { name: toTerminate.username })}
        </p>
      </Modal>

      {/* Плашка запущенных процессов */}
      {runningCount > 0 && (
        <div className="pointer-events-none fixed bottom-4 right-4 z-30 rounded-xl border border-accent/30 bg-[rgb(18_20_29/0.95)] px-3.5 py-2 text-[12px] text-mut shadow-modal backdrop-blur-xl">
          <span className="mr-1.5 inline-block h-2 w-2 animate-pulseDot rounded-full bg-accent" />
          {t("accounts.running", { n: runningCount })}
        </div>
      )}
    </div>
  );
}
