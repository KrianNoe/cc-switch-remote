import { useState } from "react";
import { toast } from "sonner";
import { Server, Plus, Trash2, Edit3, Wifi, WifiOff, TestTube } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { remoteHostApi } from "@/lib/api/remote-host";
import type { RemoteHost, ActiveRemoteInfo } from "@/lib/api/remote-host";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

const QUERY_KEY = ["remote-hosts"];
const ACTIVE_KEY = ["remote-host-active"];

function useRemoteHosts() {
  return useQuery({ queryKey: QUERY_KEY, queryFn: () => remoteHostApi.list() });
}

function useActiveRemote() {
  return useQuery({
    queryKey: ACTIVE_KEY,
    queryFn: () => remoteHostApi.getActive(),
    refetchInterval: 5000,
  });
}

interface HostFormValues {
  name: string;
  host: string;
  port: string;
  username: string;
  password: string;
}

const emptyForm = (): HostFormValues => ({
  name: "",
  host: "",
  port: "22",
  username: "",
  password: "",
});

function hostToForm(h: RemoteHost): HostFormValues {
  return {
    name: h.name,
    host: h.host,
    port: String(h.port),
    username: h.username,
    password: h.password,
  };
}

interface HostFormDialogProps {
  open: boolean;
  onClose: () => void;
  editing?: RemoteHost;
}

function HostFormDialog({ open, onClose, editing }: HostFormDialogProps) {
  const qc = useQueryClient();
  const [form, setForm] = useState<HostFormValues>(
    editing ? hostToForm(editing) : emptyForm()
  );
  const [testing, setTesting] = useState(false);

  const set = (k: keyof HostFormValues) => (e: React.ChangeEvent<HTMLInputElement>) =>
    setForm((f) => ({ ...f, [k]: e.target.value }));

  const saveMutation = useMutation({
    mutationFn: async () => {
      const port = parseInt(form.port) || 22;
      if (editing) {
        return remoteHostApi.update({ ...form, id: editing.id, port });
      }
      return remoteHostApi.create({ ...form, port });
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: QUERY_KEY });
      toast.success(editing ? "已更新远程主机" : "已添加远程主机");
      onClose();
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const handleTest = async () => {
    setTesting(true);
    try {
      const result = await remoteHostApi.testConnection(
        form.host,
        parseInt(form.port) || 22,
        form.username,
        form.password
      );
      toast.success(`连接成功: ${result}`);
    } catch (e: unknown) {
      toast.error(`连接失败: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setTesting(false);
    }
  };

  const valid = form.name && form.host && form.username && form.password;

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{editing ? "编辑远程主机" : "添加远程主机"}</DialogTitle>
        </DialogHeader>
        <div className="grid gap-3 py-2">
          <div className="grid gap-1.5">
            <Label>名称</Label>
            <Input placeholder="我的服务器" value={form.name} onChange={set("name")} />
          </div>
          <div className="grid grid-cols-3 gap-2">
            <div className="col-span-2 grid gap-1.5">
              <Label>主机 / IP</Label>
              <Input placeholder="192.168.1.100" value={form.host} onChange={set("host")} />
            </div>
            <div className="grid gap-1.5">
              <Label>端口</Label>
              <Input placeholder="22" value={form.port} onChange={set("port")} />
            </div>
          </div>
          <div className="grid gap-1.5">
            <Label>用户名</Label>
            <Input placeholder="root" value={form.username} onChange={set("username")} />
          </div>
          <div className="grid gap-1.5">
            <Label>密码</Label>
            <Input
              type="password"
              placeholder="••••••••"
              value={form.password}
              onChange={set("password")}
            />
          </div>
        </div>
        <DialogFooter className="gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={handleTest}
            disabled={testing || !form.host || !form.username || !form.password}
          >
            <TestTube className="w-3.5 h-3.5 mr-1.5" />
            {testing ? "测试中..." : "测试连接"}
          </Button>
          <Button
            size="sm"
            onClick={() => saveMutation.mutate()}
            disabled={!valid || saveMutation.isPending}
          >
            {saveMutation.isPending ? "保存中..." : "保存"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface RemoteHostsPanelProps {
  onActiveChange?: (info: ActiveRemoteInfo | null) => void;
}

export function RemoteHostsPanel({ onActiveChange }: RemoteHostsPanelProps) {
  const qc = useQueryClient();
  const { data: hosts = [], isLoading } = useRemoteHosts();
  const { data: active } = useActiveRemote();
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<RemoteHost | undefined>();
  const [deleteTarget, setDeleteTarget] = useState<RemoteHost | null>(null);

  const connectMutation = useMutation({
    mutationFn: (id: string) => remoteHostApi.connect(id),
    onSuccess: (info) => {
      qc.invalidateQueries({ queryKey: ACTIVE_KEY });
      toast.success(`已连接到 ${info.name} (${info.remote_home})`);
      onActiveChange?.(info);
    },
    onError: (e: Error) => toast.error(`连接失败: ${e.message}`),
  });

  const disconnectMutation = useMutation({
    mutationFn: () => remoteHostApi.disconnect(),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ACTIVE_KEY });
      toast.success("已断开远程连接，切换回本地模式");
      onActiveChange?.(null);
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => remoteHostApi.delete(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: QUERY_KEY });
      qc.invalidateQueries({ queryKey: ACTIVE_KEY });
      toast.success("已删除远程主机");
      setDeleteTarget(null);
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const openAdd = () => {
    setEditing(undefined);
    setFormOpen(true);
  };

  const openEdit = (h: RemoteHost) => {
    setEditing(h);
    setFormOpen(true);
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-3 border-b">
        <div className="flex items-center gap-2 text-sm font-medium">
          <Server className="w-4 h-4" />
          远程主机
        </div>
        <Button size="sm" variant="outline" onClick={openAdd}>
          <Plus className="w-3.5 h-3.5 mr-1" />
          添加
        </Button>
      </div>

      {active && (
        <div className="mx-4 mt-3 px-3 py-2 rounded-md bg-green-500/10 border border-green-500/20 text-xs flex items-center justify-between">
          <div className="flex items-center gap-2 text-green-600 dark:text-green-400">
            <Wifi className="w-3.5 h-3.5" />
            <span>
              已连接: <strong>{active.name}</strong> ({active.username}@{active.host})
            </span>
          </div>
          <Button
            size="sm"
            variant="ghost"
            className="h-6 px-2 text-xs"
            onClick={() => disconnectMutation.mutate()}
            disabled={disconnectMutation.isPending}
          >
            <WifiOff className="w-3 h-3 mr-1" />
            断开
          </Button>
        </div>
      )}

      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-2">
        {isLoading && (
          <p className="text-xs text-muted-foreground text-center py-4">加载中...</p>
        )}
        {!isLoading && hosts.length === 0 && (
          <p className="text-xs text-muted-foreground text-center py-8">
            暂无远程主机，点击「添加」开始配置
          </p>
        )}
        {hosts.map((h) => {
          const isActive = active?.host_id === h.id;
          return (
            <div
              key={h.id}
              className={`flex items-center justify-between px-3 py-2.5 rounded-md border text-sm transition-colors ${
                isActive
                  ? "border-green-500/40 bg-green-500/5"
                  : "border-border hover:bg-muted/50"
              }`}
            >
              <div className="flex items-center gap-2 min-w-0">
                <Server className={`w-3.5 h-3.5 shrink-0 ${isActive ? "text-green-500" : "text-muted-foreground"}`} />
                <div className="min-w-0">
                  <div className="font-medium truncate">{h.name}</div>
                  <div className="text-xs text-muted-foreground truncate">
                    {h.username}@{h.host}:{h.port}
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-1 shrink-0 ml-2">
                {isActive ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 px-2 text-xs text-green-600"
                    onClick={() => disconnectMutation.mutate()}
                    disabled={disconnectMutation.isPending}
                  >
                    <WifiOff className="w-3 h-3 mr-1" />
                    断开
                  </Button>
                ) : (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 px-2 text-xs"
                    onClick={() => connectMutation.mutate(h.id)}
                    disabled={connectMutation.isPending}
                  >
                    <Wifi className="w-3 h-3 mr-1" />
                    连接
                  </Button>
                )}
                <Button
                  size="icon"
                  variant="ghost"
                  className="h-7 w-7"
                  onClick={() => openEdit(h)}
                >
                  <Edit3 className="w-3.5 h-3.5" />
                </Button>
                <Button
                  size="icon"
                  variant="ghost"
                  className="h-7 w-7 text-destructive hover:text-destructive"
                  onClick={() => setDeleteTarget(h)}
                >
                  <Trash2 className="w-3.5 h-3.5" />
                </Button>
              </div>
            </div>
          );
        })}
      </div>

      <HostFormDialog
        open={formOpen}
        onClose={() => setFormOpen(false)}
        editing={editing}
      />

      {deleteTarget && (
        <ConfirmDialog
          isOpen={true}
          title="删除远程主机"
          message={`确定要删除「${deleteTarget.name}」吗？`}
          onConfirm={() => deleteMutation.mutate(deleteTarget.id)}
          onCancel={() => setDeleteTarget(null)}
        />
      )}
    </div>
  );
}
