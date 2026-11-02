#!/usr/bin/env bash
PATH=/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin:/usr/local/sbin:~/bin
export PATH

#=================================================
#	System Required: Debian 12/13 (client), CentOS/Debian/Ubuntu (server)
#	Description: ServerStatus-Plus client + server installer
#	Version: 2.0.2
#	Maintainer: ztoofar - https://github.com/ztoofar/ServerStatus-Plus
#=================================================

sh_ver="2.0.2"
filepath=$(cd "$(dirname "$0")"; pwd)
file_1=$(echo -e "${filepath}"|awk -F "$0" '{print $1}')
file="/usr/local/ServerStatus"
server_file="/usr/local/ServerStatus/server"
server_conf="/usr/local/ServerStatus/server/config.json"
client_file="/usr/local/ServerStatus/client"
client_conf="/usr/local/ServerStatus/client/config.json"
client_log_file="/tmp/serverstatus_client.log"
server_log_file="/tmp/serverstatus_server.log"
jq_file="${file}/jq"

# 下载源：全部指向本仓库
repo_owner="ztoofar"
repo_name="ServerStatus-Plus"
repo_branch="main"
repo_raw="https://raw.githubusercontent.com/${repo_owner}/${repo_name}/${repo_branch}"

Green_font_prefix="\033[32m" && Red_font_prefix="\033[31m" && Green_background_prefix="\033[42;37m" && Red_background_prefix="\033[41;37m" && Font_color_suffix="\033[0m"
Info="${Green_font_prefix}[信息]${Font_color_suffix}"
Error="${Red_font_prefix}[错误]${Font_color_suffix}"
Tip="${Green_font_prefix}[注意]${Font_color_suffix}"

Setup_proxy(){
	local proxy="$1"
	if [[ -z "$proxy" ]]; then
		proxy="${https_proxy:-${HTTPS_PROXY:-${http_proxy:-${HTTP_PROXY:-${all_proxy:-${ALL_PROXY:-}}}}}}"
	fi
	[[ -z "$proxy" ]] && return 0
	if [[ "$proxy" == socks* ]]; then
		echo -e "${Error} wget 不支持 SOCKS 代理，请使用 HTTP/HTTPS 代理地址" && exit 1
	fi
	export http_proxy="$proxy"
	export https_proxy="$proxy"
	export HTTP_PROXY="$proxy"
	export HTTPS_PROXY="$proxy"
	local safe_proxy
	safe_proxy="$(echo "$proxy" | sed -E 's#(//)[^/@]+@#\1***@#')"
	echo -e "${Info} 已启用下载代理: ${safe_proxy}"
}

#检查系统
check_sys(){
	if [[ -f /etc/redhat-release ]]; then
		release="centos"
	elif cat /etc/issue | grep -q -E -i "debian"; then
		release="debian"
	elif cat /etc/issue | grep -q -E -i "ubuntu"; then
		release="ubuntu"
	elif cat /etc/issue | grep -q -E -i "centos|red hat|redhat"; then
		release="centos"
	elif cat /proc/version | grep -q -E -i "debian"; then
		release="debian"
	elif cat /proc/version | grep -q -E -i "ubuntu"; then
		release="ubuntu"
	elif cat /proc/version | grep -q -E -i "centos|red hat|redhat"; then
		release="centos"
    fi
	bit=`uname -m`
}
check_installed_server_status(){
	[[ ! -e "${server_file}/sergate" ]] && echo -e "${Error} ServerStatus 服务端没有安装，请检查 !" && exit 1
}
check_installed_client_status(){
	[[ ! -e "${client_file}/sergate-client" ]] && echo -e "${Error} ServerStatus 客户端没有安装，请检查 !" && exit 1
}
check_pid_server(){
	PID=`ps -ef| grep -E '(^|/)sergate( |$)'| grep -v grep| grep -v ".sh"| grep -v "init.d"| grep -v "service"| awk '{print $2}'`
}
check_pid_client(){
	PID=`ps -ef| grep "sergate-client"| grep -v grep| grep -v ".sh"| grep -v "init.d"| grep -v "service"| awk '{print $2}'`
}
Download_Server_Status_server(){
	cd "/tmp"
	case "$(uname -m)" in
		x86_64|amd64) binary="sergate-linux-amd64" ;;
		aarch64|arm64) binary="sergate-linux-arm64" ;;
		*) echo -e "${Error} 不支持的 CPU 架构: $(uname -m)" && exit 1 ;;
	esac
	rm -f "sergate"
	if ! wget --no-check-certificate -O "sergate" "${repo_raw}/bin/${binary}"; then
		echo -e "${Error} ServerStatus 服务端下载失败 !" && exit 1
	fi
	[[ ! -s "sergate" ]] && echo -e "${Error} ServerStatus 服务端下载失败 !" && exit 1
	cd "${file_1}"
	[[ ! -e "${file}" ]] && mkdir "${file}"
	[[ ! -e "${server_file}" ]] && mkdir "${server_file}"
	[[ -e "${server_file}/sergate" ]] && mv "${server_file}/sergate" "${server_file}/sergate1"
	mv "/tmp/sergate" "${server_file}/sergate"
	chmod +x "${server_file}/sergate"
	if [[ ! -e "${server_file}/sergate" ]]; then
		echo -e "${Error} ServerStatus 服务端安装失败 !"
		[[ -e "${server_file}/sergate1" ]] && mv "${server_file}/sergate1" "${server_file}/sergate"
		rm -f "/tmp/sergate"
		exit 1
	fi
	[[ -e "${server_file}/sergate1" ]] && rm -f "${server_file}/sergate1"
	rm -f "/tmp/sergate"
}
Download_Server_Status_client(){
	cd "/tmp"
	case "$(uname -m)" in
		x86_64|amd64) binary="sergate-client-linux-amd64" ;;
		aarch64|arm64) binary="sergate-client-linux-arm64" ;;
		*) echo -e "${Error} 不支持的 CPU 架构: $(uname -m)" && exit 1 ;;
	esac
	rm -f "sergate-client"
	if ! wget --no-check-certificate -O "sergate-client" "${repo_raw}/bin/${binary}"; then
		echo -e "${Error} ServerStatus 客户端下载失败 !" && exit 1
	fi
	[[ ! -s "sergate-client" ]] && echo -e "${Error} ServerStatus 客户端下载失败 !" && exit 1
	cd "${file_1}"
	[[ ! -e "${file}" ]] && mkdir "${file}"
	[[ ! -e "${client_file}" ]] && mkdir "${client_file}"
	[[ -e "${client_file}/sergate-client" ]] && mv "${client_file}/sergate-client" "${client_file}/sergate-client1"
	mv "/tmp/sergate-client" "${client_file}/sergate-client"
	chmod +x "${client_file}/sergate-client"
	if [[ ! -e "${client_file}/sergate-client" ]]; then
		echo -e "${Error} ServerStatus 客户端安装失败 !"
		[[ -e "${client_file}/sergate-client1" ]] && mv "${client_file}/sergate-client1" "${client_file}/sergate-client"
		rm -f "/tmp/sergate-client"
		exit 1
	fi
	[[ -e "${client_file}/sergate-client1" ]] && rm -f "${client_file}/sergate-client1"
	rm -f "/tmp/sergate-client"
}
Service_Server_Status_server(){
	if [[ ${release} = "centos" ]]; then
		if ! wget --no-check-certificate "${repo_raw}/service/server_status_server_centos" -O /etc/init.d/status-server; then
			echo -e "${Error} ServerStatus 服务端服务管理脚本下载失败 !" && exit 1
		fi
		chmod +x /etc/init.d/status-server
		chkconfig --add status-server
		chkconfig status-server on
	else
		if ! wget --no-check-certificate "${repo_raw}/service/server_status_server_debian" -O /etc/init.d/status-server; then
			echo -e "${Error} ServerStatus 服务端服务管理脚本下载失败 !" && exit 1
		fi
		chmod +x /etc/init.d/status-server
		update-rc.d -f status-server defaults
	fi
	echo -e "${Info} ServerStatus 服务端服务管理脚本下载完成 !"
}
Service_Server_Status_client(){
	if [[ ${release} = "centos" ]]; then
		if ! wget --no-check-certificate "${repo_raw}/service/server_status_client_centos" -O /etc/init.d/status-client; then
			echo -e "${Error} ServerStatus 客户端服务管理脚本下载失败 !" && exit 1
		fi
		chmod +x /etc/init.d/status-client
		chkconfig --add status-client
		chkconfig status-client on
	else
		if ! wget --no-check-certificate "${repo_raw}/service/server_status_client_debian" -O /etc/init.d/status-client; then
			echo -e "${Error} ServerStatus 客户端服务管理脚本下载失败 !" && exit 1
		fi
		chmod +x /etc/init.d/status-client
		update-rc.d -f status-client defaults
	fi
	echo -e "${Info} ServerStatus 客户端服务管理脚本下载完成 !"
}
Write_server_config(){
	cat > ${server_conf}<<-EOF
{
  "listen": ":${server_port_s}",
  "http_listen": ":${server_http_port_s}",
  "site": {
    "title": "ServerStatus-Plus",
    "announcement": ""
  },
  "servers": [
    {
      "id": "username01",
      "username": "username01",
      "password": "password",
      "name": "Server 01",
      "type": "KVM",
      "host": "",
      "location": "Hong Kong",
      "group": "",
      "tags": [],
      "disabled": false
    }
  ]
}
EOF
}
Read_config_client(){
	if [[ ! -e "${client_conf}" ]]; then
		echo -e "${Error} ServerStatus 客户端配置文件不存在 !" && exit 1
	fi
	client_server="$(grep -o '"server"[[:space:]]*:[[:space:]]*"[^"]*"' "${client_conf}" | sed 's/.*"\([^"]*\)"$/\1/')"
	client_port="$(grep -o '"port"[[:space:]]*:[[:space:]]*[0-9]*' "${client_conf}" | grep -o '[0-9]*')"
	client_user="$(grep -o '"username"[[:space:]]*:[[:space:]]*"[^"]*"' "${client_conf}" | sed 's/.*"\([^"]*\)"$/\1/')"
	client_password="$(grep -o '"password"[[:space:]]*:[[:space:]]*"[^"]*"' "${client_conf}" | sed 's/.*"\([^"]*\)"$/\1/')"
}
Write_client_config(){
	cat > "${client_conf}" <<-EOF
{
  "server": "${server_s}",
  "port": ${server_port_s},
  "username": "${username_s}",
  "password": "${password_s}",
  "family": 0,
  "interval": 1,
  "online_check_v4": "www.aliyun.com:443",
  "online_check_v6": "www.aliyun.com:443",
  "interfaces": []
}
EOF
}
Set_server(){
	echo -e "请输入 ServerStatus 服务端的 IP/域名[server]"
	read -e -p "(默认: 127.0.0.1):" server_s
	[[ -z "$server_s" ]] && server_s="127.0.0.1"
	
	echo && echo "	================================================"
	echo -e "	IP/域名[server]: ${Red_background_prefix} ${server_s} ${Font_color_suffix}"
	echo "	================================================" && echo
}
Set_server_port(){
	while true
		do
		echo -e "请输入 ServerStatus 服务端监听的端口[1-65535]（用于服务端接收客户端消息的端口，客户端要填写这个端口）"
		read -e -p "(默认: 35601):" server_port_s
		[[ -z "$server_port_s" ]] && server_port_s="35601"
		echo $((${server_port_s}+0)) &>/dev/null
		if [[ $? -eq 0 ]]; then
			if [[ ${server_port_s} -ge 1 ]] && [[ ${server_port_s} -le 65535 ]]; then
				echo && echo "	================================================"
				echo -e "	端口: ${Red_background_prefix} ${server_port_s} ${Font_color_suffix}"
				echo "	================================================" && echo
				break
			else
				echo "输入错误, 请输入正确的端口。"
			fi
		else
			echo "输入错误, 请输入正确的端口。"
		fi
	done
}
Set_http_port(){
	while true
		do
		echo -e "请输入 ServerStatus 面板 HTTP 端口[1-65535]（浏览器访问监控面板的端口）"
		read -e -p "(默认: 8080):" server_http_port_s
		[[ -z "$server_http_port_s" ]] && server_http_port_s="8080"
		echo $((${server_http_port_s}+0)) &>/dev/null
		if [[ $? -eq 0 ]]; then
			if [[ ${server_http_port_s} -ge 1 ]] && [[ ${server_http_port_s} -le 65535 ]]; then
				echo && echo "	================================================"
				echo -e "	HTTP 端口: ${Red_background_prefix} ${server_http_port_s} ${Font_color_suffix}"
				echo "	================================================" && echo
				break
			else
				echo "输入错误, 请输入正确的端口。"
			fi
		else
			echo "输入错误, 请输入正确的端口。"
		fi
	done
}
Set_username(){
	mode=$1
	[[ -z ${mode} ]] && mode="server"
	if [[ ${mode} == "server" ]]; then
		echo -e "请输入 ServerStatus 服务端要设置的用户名[username]（字母/数字，不可与其他账号重复）"
	else
		echo -e "请输入 ServerStatus 服务端中对应配置的用户名[username]（字母/数字，不可与其他账号重复）"
	fi
	read -e -p "(默认: 取消):" username_s
	[[ -z "$username_s" ]] && echo "已取消..." && exit 0
	echo && echo "	================================================"
	echo -e "	账号[username]: ${Red_background_prefix} ${username_s} ${Font_color_suffix}"
	echo "	================================================" && echo
}
Set_password(){
	mode=$1
	[[ -z ${mode} ]] && mode="server"
	if [[ ${mode} == "server" ]]; then
		echo -e "请输入 ServerStatus 服务端要设置的密码[password]（字母/数字，可重复）"
	else
		echo -e "请输入 ServerStatus 服务端中对应配置的密码[password]（字母/数字）"
	fi
	read -e -p "(默认: changeme):" password_s
	[[ -z "$password_s" ]] && password_s="changeme"
	echo && echo "	================================================"
	echo -e "	密码[password]: ${Red_background_prefix} ${password_s} ${Font_color_suffix}"
	echo "	================================================" && echo
}
Set_name(){
	echo -e "请输入 ServerStatus 服务端要设置的节点名称[name]（支持中文，前提是你的系统和SSH工具支持中文输入，仅仅是个名字）"
	read -e -p "(默认: Server 01):" name_s
	[[ -z "$name_s" ]] && name_s="Server 01"
	echo && echo "	================================================"
	echo -e "	节点名称[name]: ${Red_background_prefix} ${name_s} ${Font_color_suffix}"
	echo "	================================================" && echo
}
Set_type(){
	echo -e "请输入 ServerStatus 服务端要设置的节点虚拟化类型[type]（例如 OpenVZ / KVM）"
	read -e -p "(默认: KVM):" type_s
	[[ -z "$type_s" ]] && type_s="KVM"
	echo && echo "	================================================"
	echo -e "	虚拟化类型[type]: ${Red_background_prefix} ${type_s} ${Font_color_suffix}"
	echo "	================================================" && echo
}
Set_location(){
	echo -e "请输入 ServerStatus 服务端要设置的节点位置[location]（支持中文，前提是你的系统和SSH工具支持中文输入）"
	read -e -p "(默认: Hong Kong):" location_s
	[[ -z "$location_s" ]] && location_s="Hong Kong"
	echo && echo "	================================================"
	echo -e "	节点位置[location]: ${Red_background_prefix} ${location_s} ${Font_color_suffix}"
	echo "	================================================" && echo
}
Set_config_server(){
	Set_username "server"
	Set_password "server"
	Set_name
	Set_type
	Set_location
}
Set_config_client(){
	Set_server
	Set_server_port
	Set_username "client"
	Set_password "client"
}
Set_ServerStatus_server(){
	check_installed_server_status
	Check_server_config
	echo && echo -e " 你要做什么？
	
 ${Green_font_prefix} 1.${Font_color_suffix} 添加 节点配置
 ${Green_font_prefix} 2.${Font_color_suffix} 删除 节点配置
————————
 ${Green_font_prefix} 3.${Font_color_suffix} 修改 节点配置 - 节点用户名
 ${Green_font_prefix} 4.${Font_color_suffix} 修改 节点配置 - 节点密码
 ${Green_font_prefix} 5.${Font_color_suffix} 修改 节点配置 - 节点名称
 ${Green_font_prefix} 6.${Font_color_suffix} 修改 节点配置 - 节点虚拟化
 ${Green_font_prefix} 7.${Font_color_suffix} 修改 节点配置 - 节点位置
 ${Green_font_prefix} 8.${Font_color_suffix} 修改 节点配置 - 全部参数
————————
 ${Green_font_prefix} 9.${Font_color_suffix} 启用/禁用 节点配置
————————
 ${Green_font_prefix}10.${Font_color_suffix} 修改 服务端监听端口" && echo
	read -e -p "(默认: 取消):" server_num
	[[ -z "${server_num}" ]] && echo "已取消..." && exit 1
	if [[ ${server_num} == "1" ]]; then
		Add_ServerStatus_server
	elif [[ ${server_num} == "2" ]]; then
		Del_ServerStatus_server
	elif [[ ${server_num} == "3" ]]; then
		Modify_ServerStatus_server_username
	elif [[ ${server_num} == "4" ]]; then
		Modify_ServerStatus_server_password
	elif [[ ${server_num} == "5" ]]; then
		Modify_ServerStatus_server_name
	elif [[ ${server_num} == "6" ]]; then
		Modify_ServerStatus_server_type
	elif [[ ${server_num} == "7" ]]; then
		Modify_ServerStatus_server_location
	elif [[ ${server_num} == "8" ]]; then
		Modify_ServerStatus_server_all
	elif [[ ${server_num} == "9" ]]; then
		Modify_ServerStatus_server_disabled
	elif [[ ${server_num} == "10" ]]; then
		Set_server_port
		Jq_edit '.listen = $listen' --arg listen ":${server_port_s}"
		echo -e "${Tip} 请自行在防火墙或云安全组中放行新端口 ${server_port_s} !"
	else
		echo -e "${Error} 请输入正确的数字[1-10]" && exit 1
	fi
	Restart_ServerStatus_server
}
List_ServerStatus_server(){
	conf_text=$(${jq_file} '.servers' ${server_conf}|${jq_file} ".[]|.username"|sed 's/\"//g')
	conf_text_total=$(echo -e "${conf_text}"|wc -l)
	[[ ${conf_text_total} = "0" ]] && echo -e "${Error} 没有发现 一个节点配置，请检查 !" && exit 1
	conf_text_total_a=$(echo $((${conf_text_total}-1)))
	conf_list_all=""
	for((integer = 0; integer <= ${conf_text_total_a}; integer++))
	do
		now_text=$(${jq_file} '.servers' ${server_conf}|${jq_file} ".[${integer}]"|sed 's/\"//g;s/,$//g'|sed '$d;1d')
		now_text_username=$(echo -e "${now_text}"|grep "username"|awk -F ": " '{print $2}')
		now_text_password=$(echo -e "${now_text}"|grep "password"|awk -F ": " '{print $2}')
		now_text_name=$(echo -e "${now_text}"|grep "name"|grep -v "username"|awk -F ": " '{print $2}')
		now_text_type=$(echo -e "${now_text}"|grep "type"|awk -F ": " '{print $2}')
		now_text_location=$(echo -e "${now_text}"|grep "location"|awk -F ": " '{print $2}')
		now_text_disabled=$(echo -e "${now_text}"|grep "disabled"|awk -F ": " '{print $2}')
		if [[ ${now_text_disabled} == "false" ]]; then
			now_text_disabled_status="${Green_font_prefix}启用${Font_color_suffix}"
		else
			now_text_disabled_status="${Red_font_prefix}禁用${Font_color_suffix}"
		fi
		conf_list_all=${conf_list_all}"用户名: ${Green_font_prefix}"${now_text_username}"${Font_color_suffix} 密码: ${Green_font_prefix}"${now_text_password}"${Font_color_suffix} 节点名: ${Green_font_prefix}"${now_text_name}"${Font_color_suffix} 类型: ${Green_font_prefix}"${now_text_type}"${Font_color_suffix} 位置: ${Green_font_prefix}"${now_text_location}"${Font_color_suffix} 状态: ${Green_font_prefix}"${now_text_disabled_status}"${Font_color_suffix}\n"
	done
	echo && echo -e "节点总数 ${Green_font_prefix}"${conf_text_total}"${Font_color_suffix}"
	echo -e ${conf_list_all}
}
Check_server_config(){
	if [[ ! -e "${server_conf}" ]]; then
		echo -e "${Error} 服务端配置文件不存在 !" && exit 1
	fi
	if [[ -x "${jq_file}" ]] && ! "${jq_file}" empty "${server_conf}" >/dev/null 2>&1; then
		echo -e "${Error} 服务端配置 JSON 格式错误，请检查 ${server_conf} !" && exit 1
	fi
}
Jq_edit(){
	local filter="$1"
	shift
	local tmp
	tmp="$(mktemp)"
	if ${jq_file} "$@" "$filter" "${server_conf}" > "${tmp}" && ${jq_file} empty "${tmp}" >/dev/null 2>&1; then
		mv "${tmp}" "${server_conf}"
	else
		rm -f "${tmp}"
		echo -e "${Error} 配置修改失败，原配置未改动 !" && exit 1
	fi
}
Add_ServerStatus_server(){
	Set_config_server
	local exists
	exists="$(${jq_file} -r --arg u "${username_s}" '.servers[]? | select(.username == $u) | .username' "${server_conf}")"
	[[ -n "${exists}" ]] && echo -e "${Error} 用户名已被使用 !" && exit 1
	Jq_edit '.servers = ([{"id": $id, "username": $username, "password": $password, "name": $name, "type": $type, "host": "None", "location": $location, "group": "", "tags": [], "disabled": false}] + (.servers // []))' \
		--arg id "${username_s}" --arg username "${username_s}" --arg password "${password_s}" --arg name "${name_s}" --arg type "${type_s}" --arg location "${location_s}"
	echo -e "${Info} 添加节点成功 ${Green_font_prefix}[ 节点名称: ${name_s}, 节点用户名: ${username_s}, 节点密码: ${password_s} ]${Font_color_suffix} !"
}
Del_ServerStatus_server(){
	List_ServerStatus_server
	[[ "${conf_text_total}" = "1" ]] && echo -e "${Error} 节点配置仅剩 1个，不能删除 !" && exit 1
	echo -e "请输入要删除的节点用户名"
	read -e -p "(默认: 取消):" del_server_username
	[[ -z "${del_server_username}" ]] && echo -e "已取消..." && exit 1
	local exists
	exists="$(${jq_file} -r --arg u "${del_server_username}" '.servers[]? | select(.username == $u) | .username' "${server_conf}")"
	[[ -z "${exists}" ]] && echo -e "${Error} 请输入正确的节点用户名 !" && exit 1
	Jq_edit '.servers |= map(select(.username != $u))' --arg u "${del_server_username}"
	echo -e "${Info} 节点删除成功 ${Green_font_prefix}[ 节点用户名: ${del_server_username} ]${Font_color_suffix}"
}
Modify_ServerStatus_server_username(){
	List_ServerStatus_server
	echo -e "请输入要修改的节点用户名"
	read -e -p "(默认: 取消):" manually_username
	[[ -z "${manually_username}" ]] && echo -e "已取消..." && exit 1
	local exists
	exists="$(${jq_file} -r --arg u "${manually_username}" '.servers[]? | select(.username == $u) | .username' "${server_conf}")"
	[[ -z "${exists}" ]] && echo -e "${Error} 请输入正确的节点用户名 !" && exit 1
	Set_username
	local new_exists
	new_exists="$(${jq_file} -r --arg u "${username_s}" '.servers[]? | select(.username == $u) | .username' "${server_conf}")"
	if [[ -n "${new_exists}" && "${username_s}" != "${manually_username}" ]]; then
		echo -e "${Error} 用户名已被使用 !" && exit 1
	fi
	Jq_edit '(.servers[] | select(.username == $u) | .username) = $new' --arg u "${manually_username}" --arg new "${username_s}"
	echo -e "${Info} 修改成功 [ 原节点用户名: ${manually_username}, 新节点用户名: ${username_s} ]"
}
Modify_ServerStatus_server_password(){
	List_ServerStatus_server
	echo -e "请输入要修改的节点用户名"
	read -e -p "(默认: 取消):" manually_username
	[[ -z "${manually_username}" ]] && echo -e "已取消..." && exit 1
	local exists
	exists="$(${jq_file} -r --arg u "${manually_username}" '.servers[]? | select(.username == $u) | .username' "${server_conf}")"
	[[ -z "${exists}" ]] && echo -e "${Error} 请输入正确的节点用户名 !" && exit 1
	Set_password
	Jq_edit '(.servers[] | select(.username == $u) | .password) = $new' --arg u "${manually_username}" --arg new "${password_s}"
	echo -e "${Info} 修改成功 [ 节点用户名: ${manually_username}, 新节点密码: ${password_s} ]"
}
Modify_ServerStatus_server_name(){
	List_ServerStatus_server
	echo -e "请输入要修改的节点用户名"
	read -e -p "(默认: 取消):" manually_username
	[[ -z "${manually_username}" ]] && echo -e "已取消..." && exit 1
	local exists
	exists="$(${jq_file} -r --arg u "${manually_username}" '.servers[]? | select(.username == $u) | .username' "${server_conf}")"
	[[ -z "${exists}" ]] && echo -e "${Error} 请输入正确的节点用户名 !" && exit 1
	Set_name
	Jq_edit '(.servers[] | select(.username == $u) | .name) = $new' --arg u "${manually_username}" --arg new "${name_s}"
	echo -e "${Info} 修改成功 [ 节点用户名: ${manually_username}, 新节点名称: ${name_s} ]"
}
Modify_ServerStatus_server_type(){
	List_ServerStatus_server
	echo -e "请输入要修改的节点用户名"
	read -e -p "(默认: 取消):" manually_username
	[[ -z "${manually_username}" ]] && echo -e "已取消..." && exit 1
	local exists
	exists="$(${jq_file} -r --arg u "${manually_username}" '.servers[]? | select(.username == $u) | .username' "${server_conf}")"
	[[ -z "${exists}" ]] && echo -e "${Error} 请输入正确的节点用户名 !" && exit 1
	Set_type
	Jq_edit '(.servers[] | select(.username == $u) | .type) = $new' --arg u "${manually_username}" --arg new "${type_s}"
	echo -e "${Info} 修改成功 [ 节点用户名: ${manually_username}, 新节点虚拟化: ${type_s} ]"
}
Modify_ServerStatus_server_location(){
	List_ServerStatus_server
	echo -e "请输入要修改的节点用户名"
	read -e -p "(默认: 取消):" manually_username
	[[ -z "${manually_username}" ]] && echo -e "已取消..." && exit 1
	local exists
	exists="$(${jq_file} -r --arg u "${manually_username}" '.servers[]? | select(.username == $u) | .username' "${server_conf}")"
	[[ -z "${exists}" ]] && echo -e "${Error} 请输入正确的节点用户名 !" && exit 1
	Set_location
	Jq_edit '(.servers[] | select(.username == $u) | .location) = $new' --arg u "${manually_username}" --arg new "${location_s}"
	echo -e "${Info} 修改成功 [ 节点用户名: ${manually_username}, 新节点位置: ${location_s} ]"
}
Modify_ServerStatus_server_all(){
	List_ServerStatus_server
	echo -e "请输入要修改的节点用户名"
	read -e -p "(默认: 取消):" manually_username
	[[ -z "${manually_username}" ]] && echo -e "已取消..." && exit 1
	local exists
	exists="$(${jq_file} -r --arg u "${manually_username}" '.servers[]? | select(.username == $u) | .username' "${server_conf}")"
	[[ -z "${exists}" ]] && echo -e "${Error} 请输入正确的节点用户名 !" && exit 1
	Set_username
	Set_password
	Set_name
	Set_type
	Set_location
	local new_exists
	new_exists="$(${jq_file} -r --arg u "${username_s}" '.servers[]? | select(.username == $u) | .username' "${server_conf}")"
	if [[ -n "${new_exists}" && "${username_s}" != "${manually_username}" ]]; then
		echo -e "${Error} 用户名已被使用 !" && exit 1
	fi
	Jq_edit '(.servers[] | select(.username == $u)) |= (.username = $username | .password = $password | .name = $name | .type = $type | .location = $location)' \
		--arg u "${manually_username}" --arg username "${username_s}" --arg password "${password_s}" --arg name "${name_s}" --arg type "${type_s}" --arg location "${location_s}"
	echo -e "${Info} 修改成功。"
}
Modify_ServerStatus_server_disabled(){
	List_ServerStatus_server
	echo -e "请输入要修改的节点用户名"
	read -e -p "(默认: 取消):" manually_username
	[[ -z "${manually_username}" ]] && echo -e "已取消..." && exit 1
	local old
	old="$(${jq_file} -r --arg u "${manually_username}" '(.servers[]? | select(.username == $u) | .disabled) // empty' "${server_conf}")"
	[[ -z "${old}" ]] && echo -e "${Error} 请输入正确的节点用户名 !" && exit 1
	Jq_edit '(.servers[] | select(.username == $u) | .disabled) |= not' --arg u "${manually_username}"
	if [[ "${old}" == "true" ]]; then
		echo -e "${Info} 修改成功 [ 原禁用状态: true, 新禁用状态: false ]"
	else
		echo -e "${Info} 修改成功 [ 原禁用状态: false, 新禁用状态: true ]"
	fi
}
Set_ServerStatus_client(){
	check_installed_client_status
	Set_config_client
	Write_client_config
	Restart_ServerStatus_client
}
Install_jq(){
	if [[ ! -e ${jq_file} ]]; then
		case "${bit}" in
			x86_64|amd64) jq_bin="jq-linux-amd64" ;;
			aarch64|arm64) jq_bin="jq-linux-arm64" ;;
			i386|i686) jq_bin="jq-linux-i386" ;;
			*) jq_bin="jq-linux-amd64" ;;
		esac
		wget --no-check-certificate "https://github.com/jqlang/jq/releases/download/jq-1.7.1/${jq_bin}" -O ${jq_file}
		[[ ! -s ${jq_file} ]] && echo -e "${Error} JQ解析器 下载失败，请检查 !" && exit 1
		chmod +x ${jq_file}
		echo -e "${Info} JQ解析器 安装完成，继续..."
	else
		echo -e "${Info} JQ解析器 已安装，继续..."
	fi
}
Install_ServerStatus_server(){
	[[ -e "${server_file}/sergate" ]] && echo -e "${Error} 检测到 ServerStatus 服务端已安装 !" && exit 1
	Set_server_port
	Set_http_port
	echo -e "${Info} 开始下载/安装..."
	Download_Server_Status_server
	Install_jq
	echo -e "${Info} 开始下载/安装 服务脚本(init)..."
	Service_Server_Status_server
	echo -e "${Info} 开始写入 配置文件..."
	Write_server_config
	echo -e "${Info} 所有步骤 安装完毕，开始启动..."
	Start_ServerStatus_server
	echo -e "${Info} 面板端口：${server_http_port_s} ，网页已内嵌到 sergate，无需单独部署 web 目录。"
}
Install_ServerStatus_client(){
	[[ -e "${client_file}/sergate-client" ]] && echo -e "${Error} 检测到 ServerStatus 客户端已安装 !" && exit 1
	check_sys
	echo -e "${Info} 开始设置 用户配置..."
	Set_config_client
	echo -e "${Info} 开始下载/安装..."
	Download_Server_Status_client
	echo -e "${Info} 开始写入 配置..."
	Write_client_config
	echo -e "${Info} 开始下载/安装 服务脚本(init)..."
	Service_Server_Status_client
	echo -e "${Info} 所有步骤 安装完毕，开始启动..."
	Start_ServerStatus_client
}
Update_ServerStatus_server(){
	check_installed_server_status
	check_pid_server
	[[ ! -z ${PID} ]] && /etc/init.d/status-server stop
	Download_Server_Status_server
	rm -rf /etc/init.d/status-server
	Service_Server_Status_server
	Start_ServerStatus_server
}
Update_ServerStatus_client(){
	check_installed_client_status
	check_pid_client
	[[ ! -z ${PID} ]] && /etc/init.d/status-client stop
	Download_Server_Status_client
	rm -rf /etc/init.d/status-client
	Service_Server_Status_client
	Start_ServerStatus_client
}
Start_ServerStatus_server(){
	check_installed_server_status
	Check_server_config
	check_pid_server
	[[ ! -z ${PID} ]] && echo -e "${Error} ServerStatus 正在运行，请检查 !" && exit 1
	/etc/init.d/status-server start
}
Stop_ServerStatus_server(){
	check_installed_server_status
	check_pid_server
	[[ -z ${PID} ]] && echo -e "${Error} ServerStatus 没有运行，请检查 !" && exit 1
	/etc/init.d/status-server stop
}
Restart_ServerStatus_server(){
	check_installed_server_status
	check_pid_server
	[[ ! -z ${PID} ]] && /etc/init.d/status-server stop
	/etc/init.d/status-server start
}
Uninstall_ServerStatus_server(){
	check_installed_server_status
	echo "确定要卸载 ServerStatus 服务端(如果同时安装了客户端，则只会删除服务端) ? [y/N]"
	echo
	read -e -p "(默认: n):" unyn
	[[ -z ${unyn} ]] && unyn="n"
	if [[ ${unyn} == [Yy] ]]; then
		check_pid_server
		[[ ! -z $PID ]] && kill -9 ${PID}
		if [[ -e "${client_file}/sergate-client" ]]; then
			rm -rf "${server_file}"
		else
			rm -rf "${file}"
		fi
		rm -rf "/etc/init.d/status-server"
		if [[ ${release} = "centos" ]]; then
			chkconfig --del status-server
		else
			update-rc.d -f status-server remove
		fi
		echo && echo "ServerStatus 卸载完成 !" && echo
	else
		echo && echo "卸载已取消..." && echo
	fi
}
Start_ServerStatus_client(){
	check_installed_client_status
	check_pid_client
	[[ ! -z ${PID} ]] && echo -e "${Error} ServerStatus 正在运行，请检查 !" && exit 1
	/etc/init.d/status-client start
}
Stop_ServerStatus_client(){
	check_installed_client_status
	check_pid_client
	[[ -z ${PID} ]] && echo -e "${Error} ServerStatus 没有运行，请检查 !" && exit 1
	/etc/init.d/status-client stop
}
Restart_ServerStatus_client(){
	check_installed_client_status
	check_pid_client
	[[ ! -z ${PID} ]] && /etc/init.d/status-client stop
	/etc/init.d/status-client start
}
Uninstall_ServerStatus_client(){
	check_installed_client_status
	echo "确定要卸载 ServerStatus 客户端(如果同时安装了服务端，则只会删除客户端) ? [y/N]"
	echo
	read -e -p "(默认: n):" unyn
	[[ -z ${unyn} ]] && unyn="n"
	if [[ ${unyn} == [Yy] ]]; then
		check_pid_client
		[[ ! -z $PID ]] && kill -9 ${PID}
		if [[ -e "${server_file}/sergate" ]]; then
			rm -rf "${client_file}"
		else
			rm -rf "${file}"
		fi
		rm -rf /etc/init.d/status-client
		if [[ ${release} = "centos" ]]; then
			chkconfig --del status-client
		else
			update-rc.d -f status-client remove
		fi
		echo && echo "ServerStatus 卸载完成 !" && echo
	else
		echo && echo "卸载已取消..." && echo
	fi
}
View_ServerStatus_client(){
	check_installed_client_status
	Read_config_client
	clear && echo "————————————————————" && echo
	echo -e "  ServerStatus 客户端配置信息：
 
  IP \t: ${Green_font_prefix}${client_server}${Font_color_suffix}
  端口 \t: ${Green_font_prefix}${client_port}${Font_color_suffix}
  账号 \t: ${Green_font_prefix}${client_user}${Font_color_suffix}
  密码 \t: ${Green_font_prefix}${client_password}${Font_color_suffix}
 
————————————————————"
}
View_client_Log(){
	[[ ! -e ${client_log_file} ]] && touch ${client_log_file}
	echo && echo -e "${Tip} 按 ${Red_font_prefix}Ctrl+C${Font_color_suffix} 终止查看日志" && echo -e "如果需要查看完整日志内容，请用 ${Red_font_prefix}cat ${client_log_file}${Font_color_suffix} 命令。" && echo
	tail -f ${client_log_file}
}
View_server_Log(){
	[[ ! -e ${server_log_file} ]] && touch ${server_log_file}
	echo && echo -e "${Tip} 按 ${Red_font_prefix}Ctrl+C${Font_color_suffix} 终止查看日志" && echo -e "如果需要查看完整日志内容，请用 ${Red_font_prefix}cat ${server_log_file}${Font_color_suffix} 命令。" && echo
	tail -f ${server_log_file}
}
Update_Shell(){
	sh_new_ver=$(wget --no-check-certificate -qO- -t1 -T3 "${repo_raw}/status.sh"|grep 'sh_ver="'|awk -F "=" '{print $NF}'|sed 's/\"//g'|head -1) && sh_new_type="github"
	[[ -z ${sh_new_ver} ]] && echo -e "${Error} 无法链接到 Github !" && exit 0
	if [[ -e "/etc/init.d/status-client" ]]; then
		rm -rf /etc/init.d/status-client
		Service_Server_Status_client
	fi
	if [[ -e "/etc/init.d/status-server" ]]; then
		rm -rf /etc/init.d/status-server
		Service_Server_Status_server
	fi
	wget -N --no-check-certificate "${repo_raw}/status.sh" && chmod +x status.sh
	echo -e "脚本已更新为最新版本[ ${sh_new_ver} ] !(注意：因为更新方式为直接覆盖当前运行的脚本，所以可能下面会提示一些报错，无视即可)" && exit 0
}
menu_client(){
while true; do
echo && echo -e "  ServerStatus 一键安装管理脚本 ${Red_font_prefix}[v${sh_ver}]${Font_color_suffix}
  -- ServerStatus-Plus | github.com/ztoofar/ServerStatus-Plus --
  
 ${Green_font_prefix} 0.${Font_color_suffix} 升级脚本
 ————————————
 ${Green_font_prefix} 1.${Font_color_suffix} 安装 客户端
 ${Green_font_prefix} 2.${Font_color_suffix} 更新 客户端
 ${Green_font_prefix} 3.${Font_color_suffix} 卸载 客户端
————————————
 ${Green_font_prefix} 4.${Font_color_suffix} 启动 客户端
 ${Green_font_prefix} 5.${Font_color_suffix} 停止 客户端
 ${Green_font_prefix} 6.${Font_color_suffix} 重启 客户端
————————————
 ${Green_font_prefix} 7.${Font_color_suffix} 设置 客户端配置
 ${Green_font_prefix} 8.${Font_color_suffix} 查看 客户端信息
 ${Green_font_prefix} 9.${Font_color_suffix} 查看 客户端日志
————————————
 ${Green_font_prefix}10.${Font_color_suffix} 切换为 服务端菜单" && echo
if [[ -e "${client_file}/sergate-client" ]]; then
	check_pid_client
	if [[ ! -z "${PID}" ]]; then
		echo -e " 当前状态: 客户端 ${Green_font_prefix}已安装${Font_color_suffix} 并 ${Green_font_prefix}已启动${Font_color_suffix}"
	else
		echo -e " 当前状态: 客户端 ${Green_font_prefix}已安装${Font_color_suffix} 但 ${Red_font_prefix}未启动${Font_color_suffix}"
	fi
else
	echo -e " 当前状态: 客户端 ${Red_font_prefix}未安装${Font_color_suffix}"
fi
echo
if ! read -e -p " 请输入数字 [0-10/q]:" num; then
	echo
	exit 0
fi
case "$num" in
	0)
	Update_Shell
	;;
	1)
	( Install_ServerStatus_client )
	continue
	;;
	2)
	( Update_ServerStatus_client )
	continue
	;;
	3)
	( Uninstall_ServerStatus_client )
	continue
	;;
	4)
	( Start_ServerStatus_client )
	continue
	;;
	5)
	( Stop_ServerStatus_client )
	continue
	;;
	6)
	( Restart_ServerStatus_client )
	continue
	;;
	7)
	( Set_ServerStatus_client )
	continue
	;;
	8)
	( View_ServerStatus_client )
	continue
	;;
	9)
	( View_client_Log )
	continue
	;;
	10)
	( menu_server )
	continue
	;;
	q|Q)
	echo "已退出"
	exit 0
	;;
	*)
	echo "请输入正确数字 [0-10/q]"
	continue
	;;
esac
done
}
menu_server(){
while true; do
echo && echo -e "  ServerStatus 一键安装管理脚本 ${Red_font_prefix}[v${sh_ver}]${Font_color_suffix}
  -- ServerStatus-Plus | github.com/ztoofar/ServerStatus-Plus --
  
 ${Green_font_prefix} 0.${Font_color_suffix} 升级脚本
 ————————————
 ${Green_font_prefix} 1.${Font_color_suffix} 安装 服务端
 ${Green_font_prefix} 2.${Font_color_suffix} 更新 服务端
 ${Green_font_prefix} 3.${Font_color_suffix} 卸载 服务端
————————————
 ${Green_font_prefix} 4.${Font_color_suffix} 启动 服务端
 ${Green_font_prefix} 5.${Font_color_suffix} 停止 服务端
 ${Green_font_prefix} 6.${Font_color_suffix} 重启 服务端
————————————
 ${Green_font_prefix} 7.${Font_color_suffix} 设置 服务端配置
 ${Green_font_prefix} 8.${Font_color_suffix} 查看 服务端信息
 ${Green_font_prefix} 9.${Font_color_suffix} 查看 服务端日志
————————————
 ${Green_font_prefix}10.${Font_color_suffix} 切换为 客户端菜单" && echo
if [[ -e "${server_file}/sergate" ]]; then
	check_pid_server
	if [[ ! -z "${PID}" ]]; then
		echo -e " 当前状态: 服务端 ${Green_font_prefix}已安装${Font_color_suffix} 并 ${Green_font_prefix}已启动${Font_color_suffix}"
	else
		echo -e " 当前状态: 服务端 ${Green_font_prefix}已安装${Font_color_suffix} 但 ${Red_font_prefix}未启动${Font_color_suffix}"
	fi
else
	echo -e " 当前状态: 服务端 ${Red_font_prefix}未安装${Font_color_suffix}"
fi
echo
if ! read -e -p " 请输入数字 [0-10/q]:" num; then
	echo
	exit 0
fi
case "$num" in
	0)
	Update_Shell
	;;
	1)
	( Install_ServerStatus_server )
	continue
	;;
	2)
	( Update_ServerStatus_server )
	continue
	;;
	3)
	( Uninstall_ServerStatus_server )
	continue
	;;
	4)
	( Start_ServerStatus_server )
	continue
	;;
	5)
	( Stop_ServerStatus_server )
	continue
	;;
	6)
	( Restart_ServerStatus_server )
	continue
	;;
	7)
	( Set_ServerStatus_server )
	continue
	;;
	8)
	( List_ServerStatus_server )
	continue
	;;
	9)
	( View_server_Log )
	continue
	;;
	10)
	( menu_client )
	continue
	;;
	q|Q)
	echo "已退出"
	exit 0
	;;
	*)
	echo "请输入正确数字 [0-10/q]"
	continue
	;;
esac
done
}
check_sys
action=""
proxy=""
while [[ $# -gt 0 ]]; do
	case "$1" in
		s|server) action="s"; shift ;;
		c|client) action="c"; shift ;;
		-p|--proxy)
			if [[ -z "${2:-}" ]]; then
				echo -e "${Error} 缺少代理地址，例如：--proxy http://127.0.0.1:7890"
				exit 1
			fi
			proxy="$2"
			shift 2
			;;
		--proxy=*) proxy="${1#*=}"; shift ;;
		*) shift ;;
	esac
done
Setup_proxy "$proxy"
if [[ "$action" == "s" ]]; then
	menu_server
elif [[ "$action" == "c" ]]; then
	menu_client
else
	menu_server
fi
